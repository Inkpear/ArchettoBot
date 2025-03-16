import logging
import os
import re
import subprocess
from json import loads
from re import sub, search
import requests
from bs4 import BeautifulSoup

logger = logging.getLogger(__name__)


class BilibiliInfo:
    """bilibili链接, Bv号解析"""

    def __init__(self, url, session):
        self.session = session

        response = self.session.get(url=url)
        response.encoding = 'utf-8'
        response.close()

        self.content = response.text
        self.soup = BeautifulSoup(self.content, 'html.parser')
        self.info = {}  # 存储解析后的视频信息
        self.video_link = []  # 存储解析后的链接信息
        self.audio_link = []  # 存储解析后的音频链接
        self.face = {}  # 存储解析后的封面链接
        self.valid_video_name = None
        try:
            self.get_video_info()
            self.get_video_link()

        except Exception as e:
            logger.exception(f"拉取{url}失败!")
            raise RuntimeError(f"视频信息获取失败: {e}") from e

    def get_video_info(self):
        data = self.soup.find("div", id="viewbox_report")
        # 获取标题
        title = data.find("h1").text.strip()
        self.info.update({"title": title})
        self.valid_video_name = sub(r'[\\/:*?"<>|\r\n]+', '_', title)
        # 获取播放量
        view = data.find("div", class_="view item").text.strip()
        self.info.update({"view": view})
        # 获取发布日期
        date = data.find("div", class_="pubdate-ip-text").text.strip()
        self.info.update({"date": date})

        data = self.soup.find("div", class_="video-toolbar-left-main")
        # 获取点赞数量
        like = data.find("span", class_="video-like-info video-toolbar-item-text").text.strip()
        self.info.update({"like": like})
        # 获取投币数量
        coin = data.find("span", class_="video-coin-info video-toolbar-item-text").text.strip()
        self.info.update({"coin": coin})
        # 获取收藏数量
        fav = data.find("span", class_="video-fav-info video-toolbar-item-text").text.strip()
        self.info.update({"fav": fav})
        # 获取转发数量
        share = data.find("div", class_="video-share-wrap video-toolbar-left-item").text.strip()
        self.info.update({"share": share})
        # 获取UP名字
        try:
            up = self.soup.find("div", class_="up-detail-top").find("a").text.strip()
        except AttributeError:
            up = self.soup.find("div", class_="staff-info").find("a", class_="staff-name").text.strip()
        self.info.update({"up": up})

        # 获取视频链接
        regx = re.search(r"https://www.bilibili.com/video/(BV[a-zA-Z0-9]{10})", self.content)
        video_url = regx.group()
        bv = regx.group(1)
        self.info.update({"bv": bv})
        self.info.update({"video_url": video_url})

    def get_video_link(self):
        data = loads(str(self.soup.find_all("script")[3].text).lstrip("window.__playinfo__=")).get("data")
        # 解析清晰度索引
        format_data = data.get("support_formats")
        format_info = {}

        for i in format_data:
            quality = i.get("quality")
            description = i.get("new_description")
            format_info.update({quality: description})

        # 解析链接索引
        link_data = data.get("dash")
        video_link = link_data.get("video")
        audio_link = link_data.get("audio")

        # 更新视频清晰度对应的链接
        for i in video_link:
            description = format_info.get(i.get("id"))
            link = i.get("baseUrl")
            self.video_link.append({
                "name": description,
                "url": link
            })
        # 更新音频链接
        self.audio_link.append({"audio": audio_link[0].get("baseUrl")})

        # 获取封面链接
        face = \
            loads(self.soup.find("div", id="app").find("script", type="application/ld+json").text).get("thumbnailUrl")[
                0]
        quality_face = search(r"^.*?\.(jpg|png)", face).group()
        self.face.update({"face": face})
        self.face.update({"quality_face": quality_face})


class BilibiliInfoServices:

    def __init__(self, path=".", cookies=None):
        self.session = requests.Session()
        self.session.headers.update({
            "User-Agent": 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 ('
                          'KHTML, like Gecko)'
                          'Chrome/121.0.0.0 Safari/537.36 Edg/121.0.0.0'
        })

        if cookies is not None:
            self.session.cookies.update(cookies)

        self.bilibili_info = None
        self.path = path
        path_list = ["face", "video", "audio"]

        for i in path_list:
            if not os.path.exists(os.path.join(self.path, i)):
                os.makedirs(os.path.join(self.path, i))

    def download_video_and_face(self, url="", quality=False, only_audio=False):
        if url:
            self.update_video_info(url)
        self.session.headers.update({
            "Referer": self.bilibili_info.info["video_url"]
        })
        logger.info(f"下载的视频清晰度为:{self.bilibili_info.video_link[0 if quality else -1].get("name")}")
        video_link = self.bilibili_info.video_link[0 if quality else -1].get("url")
        audio_link = self.bilibili_info.audio_link[0].get("audio")
        face_link = self.bilibili_info.face.get("quality_face")
        bv = self.bilibili_info.info.get("bv")
        video_temp_path = os.path.join(self.path, "video", f"{bv}-temp.mp4")
        audio_path = os.path.join(self.path, "audio", f"{bv}.mp3")
        face_path = os.path.join(self.path, "face", f"{bv}.jpg")
        video_output_path = os.path.join(self.path, "video", f"{bv}.mp4")

        if os.path.exists(video_output_path):
            logger.info("检测到缓存, 使用缓存。")
            return

        try:
            if only_audio:
                self._file_download(audio_link, audio_path)
                logger.info(f"{bv}音频下载完毕")
                return

            self._file_download(video_link, video_temp_path)
            self._file_download(audio_link, audio_path)
            self._file_download(face_link, face_path)

        except Exception as e:
            logger.exception(f"下载{url}时出错!")
            raise RuntimeError(f"下载{url}时出错!\n{e}") from e

        if not (os.path.exists(video_temp_path) or os.path.exists(audio_path) or os.path.exists(face_path)):
            logger.error("视频文件不存在!")
            raise RuntimeError("视频文件不存在!")

        # 合并视频音频
        try:
            subprocess.run(
                [
                    "ffmpeg", "-v", "16", "-i", video_temp_path, "-i", audio_path,
                    "-c:v", "copy", "-c:a", "copy", "-y", video_output_path],
                check=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
                text=True
            )
        except subprocess.CalledProcessError as e:
            logger.error(f"视频合并失败: {e.stderr}")
            raise RuntimeError(f"视频合并失败: {e.stderr}") from e

        if os.path.exists(video_output_path):
            os.remove(audio_path)
            os.remove(video_temp_path)
        logger.info(f"{self.bilibili_info.info.get('bv')}下载完毕")

    def update_video_info(self, url):
        if "http" not in url:
            url = f"https://www.bilibili.com/video/{url}"
        self.session.headers.update({
            "Referer": url
        })

        self.bilibili_info = BilibiliInfo(url, self.session)
        logger.info("获取视频信息成功!")
        return self.bilibili_info.info

    def _file_download(self, url, file_name):
        try:
            with open(file=file_name, mode='wb') as fp:
                resp = self.session.get(url, stream=True)
                resp.raise_for_status() 
                for chunk in resp.iter_content(chunk_size=8192):
                    fp.write(chunk)

        except requests.RequestException as e:
            logger.error(f"下载 {url} 失败: {e}")
            raise
    
    def set_cookie(self, cookie):
        self.session.cookies = cookie
