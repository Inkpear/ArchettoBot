#!/bin/bash
set -e

MAGENTA='\033[0;1;35;95m'
RED='\033[0;31m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

DEVICE_FILE="$SCRIPT_DIR/.napcat_device"
COMPOSE_FILE="$SCRIPT_DIR/docker-compose.yml"
CONFIG_FILE="$SCRIPT_DIR/config.yaml"

if [ ! -f "$COMPOSE_FILE" ]; then
    echo "错误: 未找到 docker-compose.yml"
    exit 1
fi

LOG_LEVEL="info"
COMMAND="up"

while [[ $# -gt 0 ]]; do
    case $1 in
        rebuild)
            COMMAND="rebuild"
            shift
            ;;
        --level)
            LOG_LEVEL="$2"
            shift 2
            ;;
        *)
            echo "用法: ./setup.sh [rebuild] [--level debug|info|warn|error]"
            exit 1
            ;;
    esac
done

export RUST_LOG="${LOG_LEVEL},headless_chrome=warn"

if [ "$COMMAND" = "rebuild" ]; then
    echo -e "${MAGENTA}重新构建 bot 容器...${NC}"
    docker compose build bot 2>&1
    echo -e "${MAGENTA}重启 bot 容器 ...${NC}"
    docker compose up -d bot --no-deps 2>&1
    echo -e "${MAGENTA}查看日志: docker logs -f archetto-bot${NC}"
    exit 0
fi

# --- 验证 config.yaml ---
if [ ! -f "$CONFIG_FILE" ]; then
    echo -e "${RED}错误: 未找到 config.yaml${NC}"
    if [ -f config.example.yaml ]; then
        echo "请从 config.example.yaml 复制模板: cp config.example.yaml config.yaml"
        echo "然后编辑 config.yaml 填入你的配置"
    fi
    exit 1
fi

echo -e "${MAGENTA}读取 config.yaml 配置...${NC}"

# Parse ws section from config.yaml (simple YAML subset, no external deps)
WS_HOST=$(sed -n '/^ws:/,/^[a-z]/p' "$CONFIG_FILE" | grep '^\s*host:' | sed 's/.*"\(.*\)".*/\1/;s/.*: *//' | tr -d '"' | xargs)
WS_PORT=$(sed -n '/^ws:/,/^[a-z]/p' "$CONFIG_FILE" | grep '^\s*port:' | grep -o '[0-9]*')
WS_TOKEN=$(sed -n '/^ws:/,/^[a-z]/p' "$CONFIG_FILE" | grep '^\s*access_token:' | sed 's/.*"\(.*\)".*/\1/;s/.*: *//' | tr -d '"' | xargs)

if [ -z "$WS_PORT" ]; then
    echo -e "${RED}错误: config.yaml 中未找到 ws.port${NC}"
    exit 1
fi
if [ -z "$WS_TOKEN" ]; then
    echo -e "${RED}错误: config.yaml 中未找到 ws.access_token${NC}"
    exit 1
fi

echo "  WS Host:   ${WS_HOST:-0.0.0.0}"
echo "  WS Port:   $WS_PORT"
echo "  WS Token:  $WS_TOKEN"

# --- 自动配置 NapCat OneBot 协议 ---
ONEBOT_FILES=$(ls napcat-data/config/onebot11_*.json 2>/dev/null || true)
if [ -z "$ONEBOT_FILES" ]; then
    echo -e "${RED}错误: 未找到 napcat-data/config/onebot11_*.json${NC}"
    echo "请确保已放置 NapCat 的 OneBot11 配置文件"
    exit 1
fi

echo -e "${MAGENTA}配置 NapCat OneBot 连接协议...${NC}"
for ONEBOT_FILE in $ONEBOT_FILES; do
    echo "  更新: $ONEBOT_FILE"
    WS_PORT="$WS_PORT" WS_TOKEN="$WS_TOKEN" ONEBOT_FILE="$ONEBOT_FILE" python3 -c "
import json, os

ws_port = os.environ['WS_PORT']
ws_token = os.environ['WS_TOKEN']
fp = os.environ['ONEBOT_FILE']

with open(fp) as f:
    cfg = json.load(f)

net = cfg.setdefault('network', {})

# WebSocket client: NapCat connects to bot
for ws in net.get('websocketClients', []):
    ws['url'] = f'ws://host.docker.internal:{ws_port}'
    ws['token'] = ws_token
    ws['enable'] = True
    print(f'    -> websocketClient: {ws[\"url\"]}')

# HTTP server: bot calls NapCat API
for http in net.get('httpServers', []):
    http['token'] = ws_token

with open(fp, 'w') as f:
    json.dump(cfg, f, indent=2, ensure_ascii=False)

print('    NapCat OneBot 配置已更新')
"
done

# --- up 命令 ---
if [ ! -f "$DEVICE_FILE" ]; then
    MAC=$(printf '02:%02x:%02x:%02x:%02x:%02x' $((RANDOM%256)) $((RANDOM%256)) $((RANDOM%256)) $((RANDOM%256)) $((RANDOM%256)))
    cat > "$DEVICE_FILE" <<EOF
NAPCAT_MAC_ADDRESS=$MAC
NAPCAT_HOSTNAME=napcat-qq
EOF
    echo -e "${MAGENTA}已生成设备标识:${NC}"
    cat "$DEVICE_FILE"
    echo ""
fi

source "$DEVICE_FILE"

# Docker creates a directory for missing bind-mount files, which breaks SQLite.
# Ensure bot.db exists as a regular file before starting containers.
if [ ! -f bot.db ]; then
    rm -rf bot.db
    touch bot.db
fi

echo -e "${MAGENTA}启动容器 (日志级别: $LOG_LEVEL)...${NC}"
export NAPCAT_MAC_ADDRESS
export NAPCAT_HOSTNAME

docker compose up -d 2>&1

echo -e "${MAGENTA}等待 NapCat 就绪...${NC}"
for i in $(seq 1 30); do
    TOKEN=$(docker exec napcat cat /app/napcat/config/webui.json 2>/dev/null | python3 -c "import json,sys; d=json.load(sys.stdin); print(d['token'])" 2>/dev/null) || true
    if [ -n "$TOKEN" ]; then
        break
    fi
    sleep 2
done

if [ -z "$TOKEN" ]; then
    echo -e "${MAGENTA}WebUI 地址: http://localhost:6099/webui${NC}"
else
    echo ""
    echo -e "${MAGENTA}WebUI: http://localhost:6099/webui?token=$TOKEN${NC}"
    echo -e "${MAGENTA}日志: docker logs -f archetto-bot${NC}"
fi
