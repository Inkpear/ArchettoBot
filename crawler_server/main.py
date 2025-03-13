from get_competition import RecentContestServices
from get_bilibili_info import BilibiliInfoServices
from fastapi import FastAPI
from fastapi.responses import JSONResponse
import logging
import uvicorn
import os

app = FastAPI()

if not os.path.exists(os.path.join(".", "logs")):
    os.makedirs(os.path.join(".", "logs"))

logging.basicConfig(
    level=logging.INFO,
    format='[crawler][%(asctime)s][%(levelname)s]:%(message)s',
    handlers=[logging.StreamHandler(), logging.FileHandler(os.path.join(".", "logs", "crawler.log"), encoding='utf-8')]
)

cpt_services = RecentContestServices()
bili_services = BilibiliInfoServices(path=os.path.join("data"))


@app.get("/get_competition_info/{_type}")
async def get_competition_info(_type: str):
    data = []
    try:
        if _type == "all":
            data += cpt_services.get_nowcoder_contests()
            data += cpt_services.get_luogu_contests()
            data += cpt_services.get_atcoder_contests()
            data += cpt_services.get_codeforces_contests()
            data += cpt_services.get_lanqiao_contests()
            data += cpt_services.get_leetcode_contests()
        elif _type == "nowcoder":
            data += cpt_services.get_nowcoder_contests()
        elif _type == "codeforces":
            data += cpt_services.get_codeforces_contests()
        elif _type == "atcoder":
            data += cpt_services.get_atcoder_contests()
        elif _type == "leetcode":
            data += cpt_services.get_leetcode_contests()
        elif _type == "luogu":
            data += cpt_services.get_luogu_contests()
        elif _type == "lanqiao":
            data += cpt_services.get_lanqiao_contests()
        else:
            return JSONResponse(status_code=400, content={
                "messages": "错误的请求参数!"
            }
        )
    except Exception as e:
        return JSONResponse(status_code=502,content={
                "messages": f"获取比赛信息失败!",
            }
        )

    logging.info("获取比赛信息成功!")

    return JSONResponse(status_code=200,content=data)


@app.get("/get_bilibili_info")
async def get_bilibili_info(
        bv: str,
        only_info: bool = False,
        only_audio: bool = False,
):
    try:
        video_info = bili_services.update_video_info(bv)
        if not only_info:
            bili_services.download_video_and_face(only_audio=only_audio)
    except Exception as _:
        return JSONResponse(status_code=502, content={
            "message": "获取视频信息发生错误!",
        })
    return JSONResponse(status_code=200, content=video_info)

if __name__ == "__main__":
    uvicorn.run("main:app", host="localhost", port=8086, reload=True)
