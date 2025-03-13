from pydantic import BaseModel


class Contest(BaseModel):
    name: str
    start_time: int
    duration: int
    platform: str
    link: str


class VideoInfo(BaseModel):
    title: str
    view: str
    date: str
    like: str
    coin: str
    fav: str
    share: str
    up: str
    bv: str
    link: str


class Response(BaseModel):
    code: int
    data: list[Contest] | VideoInfo | None
    message: str | None = None
    timestamp: str


def to_contest(data) -> Contest:
    return Contest(
        name=data["name"],
        start_time=data["start_time"],
        duration=data["duration"],
        platform=data["platform"],
        link=data["link"],
    )


def to_video_info(data) -> VideoInfo:
    return VideoInfo(
        title=data["title"],
        view=data["view"],
        date=data["date"],
        like=data["like"],
        coin=data["coin"],
        fav=data["fav"],
        share=data["share"],
        up=data["up"],
        bv=data["bv"],
        link=data["video_url"]
    )
