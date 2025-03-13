import requests
from bs4 import BeautifulSoup
import re
from datetime import datetime
import pytz
import logging

logger = logging.getLogger(__name__)


def _get_midnight_seconds():
    """获取当日UTC零时的时间戳"""
    now = datetime.now(pytz.utc)
    midnight = now.replace(hour=0, minute=0, second=0, microsecond=0)
    return int(midnight.timestamp())


class RecentContestServices:
    def __init__(self, day=7):
        self.leetcode_url = "https://leetcode.cn/graphql"
        self.atcoder_url = "https://atcoder.jp/contests/"
        self.codeforces_url = "https://mirror.codeforces.com/api/contest.list?gym=false"
        self.luogu_url = "https://www.luogu.com.cn/contest/list?page=1&_contentOnly=1"
        self.lanqiao_url = ("https://www.lanqiao.cn/api/v2/contests/?sort=opentime&paginate=0&status=not_finished"
                            "&game_type_code=2")
        self.nowcoder_url = "https://ac.nowcoder.com/acm/contest/vip-index"
        self.query_end_seconds = day * 24 * 3600
        self.midnight_seconds = _get_midnight_seconds()

    def _is_intime(self, start_time, duration):
        """判断比赛时间是否在查询范围内"""
        end_time = start_time + duration
        if start_time > self.midnight_seconds + self.query_end_seconds or duration >= 24 * 3600:
            return 1  # 超出最晚时间或持续时间过长
        if end_time < self.midnight_seconds:
            return 2  # 已结束
        return 0

    def get_leetcode_contests(self):
        """获取力扣比赛"""
        query = """
        {
            contestUpcomingContests {
                title
                startTime
                duration
                titleSlug
            }
        }
        """
        try:
            response = requests.post(self.leetcode_url, json={'query': query})
            response.raise_for_status()
            data = response.json()
            contests = []
            for item in data["data"]["contestUpcomingContests"][:2]:  # 取最近两场
                name = item["title"]
                start_time = item["startTime"]
                duration = item["duration"]
                link = f"https://leetcode.cn/contest/{item['titleSlug']}"

                if self._is_intime(start_time, duration) != 0:
                    continue
                contests.append({
                    "name": name,
                    "start_time": start_time,
                    "duration": duration,
                    "platform": "力扣",
                    "link": link
                })
            return contests
        except Exception as e:
            logger.exception(f"Error fetching LeetCode contests: {e}")
            raise RuntimeError("获取力扣比赛失败！") from e

    def get_codeforces_contests(self):
        """获取Codeforces比赛"""
        try:
            response = requests.get(self.codeforces_url)
            response.raise_for_status()
            data = response.json()
            contests = []
            for item in sorted(data["result"],
                               key=lambda x: -x["startTimeSeconds"]):  # 按开始时间倒序
                start_time = item["startTimeSeconds"]
                duration = item["durationSeconds"]

                if self._is_intime(start_time, duration) != 0:
                    continue
                contests.append({
                    "name": item["name"],
                    "start_time": start_time,
                    "duration": duration,
                    "platform": "Codeforces",
                    "link": f"https://mirror.codeforces.com/contests/{item['id']}"
                })
            return contests
        except Exception as e:
            logger.exception(f"Error fetching Codeforces contests: {e}")
            raise RuntimeError("获取cf比赛失败！") from e

    def get_nowcoder_contests(self):
        """获取牛客比赛"""
        try:
            response = requests.get(self.nowcoder_url)
            response.raise_for_status()
            soup = BeautifulSoup(response.text, 'html.parser')
            contests = []

            for item in soup.select(".platform-item-main"):
                title = item.select_one("a").text.strip()
                link = "https://ac.nowcoder.com" + item.select_one("a")["href"]
                time_text = item.select_one(".match-time-icon").text

                # 提取时间字符串
                time_matches = re.findall(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}", time_text)
                if len(time_matches) < 2:
                    continue

                # 转换为时间戳
                start_time = int(datetime.strptime(time_matches[0], "%Y-%m-%d %H:%M").timestamp())
                end_time = int(datetime.strptime(time_matches[1], "%Y-%m-%d %H:%M").timestamp())
                duration = end_time - start_time

                if self._is_intime(start_time, duration) != 0:
                    continue
                contests.append({
                    "name": title,
                    "start_time": start_time,
                    "duration": duration,
                    "platform": "牛客",
                    "link": link
                })
            return contests
        except Exception as e:
            logger.exception(f"Error fetching Nowcoder contests: {e}")
            raise RuntimeError("获取牛客比赛失败！") from e

    def get_atcoder_contests(self):
        """获取AtCoder比赛"""
        try:
            response = requests.get(self.atcoder_url)
            response.raise_for_status()
            soup = BeautifulSoup(response.text, 'html.parser')
            contests = []

            table = soup.find("div", id="contest-table-upcoming")
            if not table:
                return []

            for row in table.select("tr")[1:]:  # 跳过表头
                cols = row.select("td")
                if len(cols) < 4:
                    continue

                # 解析比赛信息
                title = cols[1].text.strip()
                link = "https://atcoder.jp" + cols[1].find("a")["href"]

                # 处理比赛时间（含时区）
                time_str = cols[0].text.strip()
                dt = datetime.strptime(time_str, "%Y-%m-%d %H:%M:%S%z")
                start_time = int(dt.timestamp())

                # 解析持续时间
                duration_str = cols[2].text.strip()
                h, m = map(int, duration_str.split(":"))
                duration = h * 3600 + m * 60

                # 处理比赛名称
                if "（" in title:
                    title = title.split("（")[1].split("）")[0]
                title = re.sub(f".\\n.\\n", "", title)

                if self._is_intime(start_time, duration) != 0:
                    continue
                contests.append({
                    "name": title,
                    "start_time": start_time,
                    "duration": duration,
                    "platform": "AtCoder",
                    "link": link
                })
            return contests
        except Exception as e:
            logger.exception(f"Error fetching AtCoder contests: {e}")
            raise RuntimeError("获取牛客比赛失败") from e

    def get_luogu_contests(self, is_rated=True):
        """获取洛谷比赛"""
        try:
            headers = {
                "X-Requested-With": "XMLHttpRequest",
                "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) "
                              "Chrome/125.0.0.0 Safari/537.36",
                "Referer": "https://www.luogu.com.cn/contest/list"
            }

            params = {
                "page": 1,
                "_contentOnly": 1
            }

            response = requests.get(self.luogu_url, headers=headers, params=params)
            response.raise_for_status()

            data = response.json()
            contest_data = data["currentData"]["contests"]["result"]

            contests = []
            for item in contest_data:
                if is_rated and not item.get("rated", False):
                    continue

                # 处理比赛时间
                start_time = item["startTime"]
                duration = item["endTime"] - start_time

                if self._is_intime(start_time, duration) != 0:
                    continue

                contests.append({
                    "name": item["name"],
                    "start_time": start_time,
                    "duration": duration,
                    "platform": "洛谷",
                    "link": f"https://www.luogu.com.cn/contest/{item['id']}"
                })
            return contests
        except Exception as e:
            logger.exception(f"Error fetching Luogu contests: {e}")
            raise RuntimeError("获取洛谷比赛失败！") from e

    def get_lanqiao_contests(self):
        """获取蓝桥杯比赛"""
        try:
            response = requests.get(self.lanqiao_url)
            response.raise_for_status()
            contests = []

            for item in response.json():
                # 解析时间（ISO格式含时区）
                start_dt = datetime.fromisoformat(item["open_at"].replace("Z", "+00:00"))
                end_dt = datetime.fromisoformat(item["end_at"].replace("Z", "+00:00"))

                contests.append({
                    "name": item["name"],
                    "start_time": int(start_dt.timestamp()),
                    "duration": int(end_dt.timestamp()) - int(start_dt.timestamp()),
                    "platform": "蓝桥云课",
                    "link": f"https://www.lanqiao.cn{item['html_url']}"
                })
            return [c for c in contests if self._is_intime(c["start_time"], c["duration"]) == 0]
        except Exception as e:
            logger.exception(f"Error fetching Lanqiao contests: {e}")
            raise RuntimeError("获取蓝桥比赛失败") from e
