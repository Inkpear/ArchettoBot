from get_competition import RecentContestServices
from get_bilibili_info import BilibiliInfoServices
from fastapi import FastAPI
from datetime import datetime
from converter import *
import logging
import re

app = FastAPI()

logging.basicConfig(
    level=logging.INFO,
    format='[crawler][%(asctime)s][%(levelname)s]:%(message)s',
    handlers=[logging.StreamHandler(), logging.FileHandler("crawler.log", encoding='utf-8')]
)

cpt_services = RecentContestServices()
bili_services = BilibiliInfoServices()


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
            return Response(
                code=400,
                data=None,
                messages="错误的请求参数!",
                timestamp=datetime.now().isoformat()
            )
    except Exception as e:
        return Response(
            code=400,
            data=None,
            messages=f"请求发生错误 {e}",
            timestamp=datetime.now().isoformat()
        )

    contests = [to_contest(i) for i in data]
    logging.info("获取比赛信息成功!")

    return Response(
        code=200,
        data=contests,
        timestamp=datetime.now().isoformat()
    )


@app.get("/get_bilibili_info/")
async def get_bilibili_info(
        bv: str,
        only_info: bool = False,
        only_audio: bool = False,
):
    if not re.match(r"BV[a-zA-Z0-9]{10}", bv):
        return Response(
            code=400,
            data=None,
            message="不合法的bv号!",
            timestamp=datetime.now().isoformat()
        )
    try:
        video_info = to_video_info(bili_services.update_video_info(bv))
        if not only_info:
            bili_services.download_video_and_face(only_audio=only_audio)
    except Exception as _:
        return Response(
            code=400,
            data=None,
            message="获取视频信息发生错误!",
            timestamp=datetime.now().isoformat()
        )

    return Response(
        code=200,
        data=video_info,
        timestamp=datetime.now().isoformat()
    )
