import asyncio
from get_competition import RecentContestServices
from get_bilibili_info import BilibiliInfoServices
from fastapi import FastAPI, HTTPException
from fastapi.responses import JSONResponse
from pydantic import BaseModel
import logging
import uvicorn
import os
import yaml

app = FastAPI()

log_path = os.path.join(".", "logs")
config_path = os.path.join("config.yaml")

if not os.path.exists(log_path):
    os.makedirs(log_path)

logging.basicConfig(
    level=logging.INFO,
    format='[crawler][%(asctime)s][%(levelname)s]:%(message)s',
    handlers=[logging.StreamHandler(), logging.FileHandler(os.path.join(".", "logs", "crawler.log"), encoding='utf-8')]
)

if not os.path.exists(config_path):
    logging.warning("配置文件不存在, 已退出")
    exit(0)

config = None

with open(config_path, 'r') as f:
    config = yaml.load(f, Loader=yaml.FullLoader)

cpt_services = RecentContestServices()
bili_services = BilibiliInfoServices(path=os.path.join("data"))


@app.get("/get_competition_info/{_type}")
async def get_competition_info(_type: str):
    try:
        if _type == "all":
            tasks = [
                cpt_services.get_nowcoder_contests(),
                cpt_services.get_luogu_contests(),
                cpt_services.get_atcoder_contests(),
                cpt_services.get_codeforces_contests(),
                cpt_services.get_lanqiao_contests(),
                cpt_services.get_leetcode_contests()
            ]
            results = await asyncio.gather(*tasks, return_exceptions=True)
            data = []
            for result in results:
                if isinstance(result, Exception):
                    continue
                data += result
        elif _type == "nowcoder":
            data = await cpt_services.get_nowcoder_contests()
        elif _type == "luogu":
            data = await cpt_services.get_luogu_contests()
        elif _type == "atcoder":
            data = await cpt_services.get_atcoder_contests()
        elif _type == "codeforces":
            data = await cpt_services.get_codeforces_contests()
        elif _type == "leetcode":
            data = await cpt_services.get_leetcode_contests()
        elif _type == "lanqiao":
            data = await cpt_services.get_lanqiao_contests()
        else:
            raise HTTPException(status_code=400, detail="错误的请求参数!")
    except Exception as e:
        logging.error(f"API Error: {str(e)}")
        raise HTTPException(status_code=502, detail="获取比赛信息失败")

    logging.info("获取比赛信息成功!")
    return data

class BiliData(BaseModel):
    bv: str
    cookie: str | None
    quality: bool
    only_info: bool
    only_audio: bool

@app.post("/get_bilibili_info")
async def get_bilibili_info(bili_data: BiliData):
    bv = bili_data.bv
    cookie = bili_data.cookie
    only_info = bili_data.only_info
    only_audio = bili_data.only_audio
    quality = bili_data.quality

    if (cookie):
        bili_services.set_cookie(cookie)
    try:
        video_info = bili_services.update_video_info(bv)
        if not only_info:
            bili_services.download_video_and_face(only_audio=only_audio, quality=quality)
    except RuntimeError as _:
        return JSONResponse(status_code=502, content={
            "message": "获取视频信息发生错误!",
        })

    except Exception as e:
        logging.exception(e)
        return JSONResponse(status_code=502, content={
            "message": "获取视频信息发生未知错误!",
        })
    
    return JSONResponse(status_code=200, content=video_info)

if __name__ == "__main__":
    host, port = config['crawler_server_addr']
    uvicorn.run("main:app", host=host, port=port, reload=True)
