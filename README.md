# ArchettoBot

算法竞赛信息 QQ 机器人，基于 NapCat + OneBot 11 协议。

## 架构

```
crates/
├── napcat-sdk/   # NapCat OneBot 11 WebSocket SDK
├── crawler/       # 多平台竞赛爬虫 (LeetCode/Codeforces/NowCoder/AtCoder/Luogu)
└── bot-core/      # 机器人核心 — 事件处理、命令、定时任务、卡片渲染
```

## 运行环境

- Rust 1.80+
- NapCatQQ（正向 WebSocket 模式连接到 bot）

## 快速开始

1. 复制配置文件并修改：
```
cp config.example.yaml config.yaml
```

2. 确保 NapCat 配置为正向 WS 连接到 `ws://host:port/?access_token=your-token`

3. 启动：
```
cargo run --release
```

## Docker 部署

使用 `setup.sh` 管理容器：

```
./setup.sh              # 首次启动（生成设备标识 + 启动 napcat + bot）
./setup.sh rebuild      # 重新构建并重启 bot 容器
./setup.sh --level debug # 以 debug 日志启动
```

`setup.sh` 会自动处理：
- 随机生成 NapCat MAC 地址和 hostname（写入 `.napcat_device`）
- 等待 NapCat WebUI 就绪后打印访问地址和 token
- `rebuild` 仅重建 bot 镜像并热重启，不动 NapCat 容器

## 数据库

SQLite (`bot.db`)，WAL 模式，启动时自动创建并迁移。

## 命令

| 命令 | 权限 | 说明 |
|------|------|------|
| `/查比赛 [n\|all]` | 需开启 competition 功能 | 查询即将开始的比赛 |
| B站链接/BV号 | 需开启 bili_parse 功能 | 自动解析视频信息 |
| `/添加管理 <qq>` | Master | 添加管理员 |
| `/删除管理 <qq>` | Master | 移除管理员 |
| `/set_config <功能> <t/f>` | Admin+ | 开关群/私聊功能 |
| `/set_config 通知 <msg>` | Admin+ | 设置入群欢迎词 |
| `/heart_beat <t/f>` | Master | 开关心跳 |

## 比赛通知

每天 2:00 UTC 更新比赛数据，赛前 1 小时通过 tick（每 5 分钟检查）自动推送通知卡片。

