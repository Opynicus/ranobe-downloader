import os.path
import shutil
import time
import logging

import requests
from bs4 import BeautifulSoup
import aiohttp
import asyncio
import hashlib
from config import Config

Config.init()

cookies = {
    "ews_key": str(Config.ews_key()),
    "ews_token": str(Config.ews_token())
}

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(message)s')
logger = logging.getLogger(__name__)

TEMPLATE_PREFIX = ("<?xml version='1.0' encoding='utf-8'?><html xmlns=\"http://www.w3.org/1999/xhtml\"><head><meta "
                   "http-equiv=\"X-UA-Compatible\" content=\"IE=edge\"/>")
TEMPLATE_MIDDLE = ("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"/><link rel=\"stylesheet\" "
                   "type=\"text/css\" media=\"screen\" href=\"main.css\"/><script src=\"main.js\"></script><meta "
                   "http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\"/></head>")
TEMPLATE_SUFFIX = "</body></html>"
CONF_PREFIX = ("<?xml version=\"1.0\"  encoding=\"UTF-8\"?><package xmlns=\"http://www.idpf.org/2007/opf\" "
               "version=\"2.0\" unique-identifier=\"uuid_id\">  <metadata xmlns:opf=\"http://www.idpf.org/2007/opf\" "
               "xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:dcterms=\"http://purl.org/dc/terms/\" "
               "xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" "
               "xmlns:calibre=\"http://calibre.kovidgoyal.net/2009/metadata\"><dc:title>")

ROOT = Config.esj_root()

NOVEL_PATH = os.path.join(ROOT, "esjNovelRaw")
OUTPUT_Novel_PATH = os.path.join(ROOT, "esjNovel")

illustration_url_list = []

illustration_url_mp = {}

novel_episode_mp: dict[str, str] = {}

policy = asyncio.WindowsSelectorEventLoopPolicy()
asyncio.set_event_loop_policy(policy)
# 图片下载最大协程数
semaphore = asyncio.Semaphore(400)

def gen_content_opf(title, author):
    content = CONF_PREFIX
    content += title
    content += "</dc:title><dc:creator>" + author + "</dc:creator>"
    content += "<meta name=\"cover\" content=\"cover.jpg\"/>"
    content += "</metadata><manifest>"
    content += "<item id=\"titlepage.xhtml\" href=\"Text/titlepage.xhtml\" media-type=\"application/xhtml+xml\" />"
    title_text_dir = os.path.join(NOVEL_PATH, title, "OEBPS", "Text")
    cnt = 0
    for text in os.listdir(title_text_dir):
        with open(os.path.join(title_text_dir, text), "r", encoding="utf-8") as f:
            c = f.read()
        with open(os.path.join(title_text_dir, text), "w", encoding="utf-8") as f:
            for k in illustration_url_mp:
                c = c.replace(k, os.path.join("../", "Images", illustration_url_mp.get(k)))
            f.write(c)
        content += "<item id=\""
        content += str(cnt) + ".xhtml"
        content += "\" href=\"Text/"
        content += str(cnt) + ".xhtml"
        content += "\" media-type=\"application/xhtml+xml\"/>"
        cnt += 1
    content += "<item id=\"ncx\" href=\"toc.ncx\" media-type=\"application/x-dtbncx+xml\"/>"
    title_images_dir = os.path.join(NOVEL_PATH, title, "OEBPS", "Images")
    cnt = 0
    for pic in os.listdir(title_images_dir):
        content += "<item id=\"added"
        content += str(cnt)
        content += "\" href=\""
        content += "Images/" + pic
        content += "\" media-type=\"image/jpeg\"/>"
        cnt += 1
    content += "<item id=\"ncx\" href=\"toc.ncx\" media-type=\"application/x-dtbncx+xml\"/>"
    content += "</manifest><spine toc=\"ncx\"><itemref idref=\"titlepage.xhtml\" />"
    for idx in range(0, len(os.listdir(title_text_dir))):
        content += "<itemref idref=\""
        content += str(idx) + ".xhtml"
        content += "\"/>"
    content += "</spine><guide><reference type=\"cover\" href=\"titlepage.xhtml\" title=\"Cover\"/></guide></package>"
    with open(os.path.join(NOVEL_PATH, title, "OEBPS", "content.opf"), "w", encoding="utf-8") as conf:
        conf.write(content)
    shutil.copy(os.path.join(ROOT, "resources", "container.xml"), os.path.join(NOVEL_PATH, title, "META-INF"))
    shutil.copy(os.path.join(ROOT, "resources", "mimetype"), os.path.join(NOVEL_PATH, title))


async def gen_multi_novels(novel_dir_url_list):
    if os.path.exists(OUTPUT_Novel_PATH):
        shutil.rmtree(OUTPUT_Novel_PATH)
    os.mkdir(OUTPUT_Novel_PATH)
    tasks = []
    for novel_dir_url in novel_dir_url_list:
        task = asyncio.create_task(get_novel(novel_dir_url))
        tasks.append(task)
    await asyncio.wait(tasks, timeout=None)


async def get_novel(novel_dir_url):
    if not os.path.exists(NOVEL_PATH):
        os.mkdir(NOVEL_PATH)
    async with aiohttp.ClientSession() as session:
        async with session.get(novel_dir_url, cookies=cookies) as res:
            result = await res.text()
    soup = BeautifulSoup(result, "lxml")
    title = soup.find(name="h2", class_="p-t-10 text-normal").text
    author = soup.find(name="ul", class_="list-unstyled mb-2 book-detail").find(name="a").text
    title_dir = os.path.join(NOVEL_PATH, title)
    pic_path = os.path.join(title_dir, "OEBPS", "Images")
    if os.path.exists(title_dir):
        shutil.rmtree(title_dir)
    os.mkdir(title_dir)
    os.mkdir(os.path.join(title_dir, "OEBPS"))
    os.mkdir(os.path.join(title_dir, "OEBPS", "FONTS"))
    os.mkdir(os.path.join(title_dir, "OEBPS", "Images"))
    os.mkdir(os.path.join(title_dir, "OEBPS", "STYLES"))
    os.mkdir(os.path.join(title_dir, "OEBPS", "Text"))
    os.mkdir(os.path.join(title_dir, "META-INF"))

    shutil.copyfile(os.path.join(ROOT, "resources", "default_cover.jpg"), os.path.join(pic_path, "cover.jpg"))
    try:
        cover_url = soup.find(name="div", class_="product-gallery text-center mb-3").find(name="a").get("href")
        cover_res = requests.get(cover_url, cookies=cookies).content
        with open(os.path.join(pic_path, "cover.jpg"), "wb") as pic:
            pic.write(cover_res)
        logging.info(f"保存封面: {cover_url}成功")
    except AttributeError:
        logging.info(f"{title}小说封面不存在，将使用默认封面")

    chapterList = soup.find(name="div", id="chapterList").find_all(name={"a"})
    tasks = []
    for idx, chapter in enumerate(chapterList):
        task = asyncio.create_task(get_episode(chapter.get("href"), idx, title))
        tasks.append(task)
    await asyncio.wait(tasks, timeout=None)

    if len(illustration_url_list) != 0:
        tasks = []
        for illustration_url in illustration_url_list:
            if title not in illustration_url or len(illustration_url[title]) == 0:
                continue
            task = asyncio.create_task(fetch_episode_art(illustration_url, title))
            tasks.append(task)
        if tasks:
            await asyncio.wait(tasks, timeout=None)

    gen_content_opf(title, author)
    create_title_page(title_dir)
    create_toc(title, author)
    logging.info(f"开始打包:《{title}》")
    shutil.make_archive(os.path.join(NOVEL_PATH, title), "zip", os.path.join(NOVEL_PATH, title))
    os.rename(os.path.join(NOVEL_PATH, title + ".zip"), os.path.join(OUTPUT_Novel_PATH, title + ".epub"))
    logging.info(f"打包:《{title}》完成！")


async def get_episode(episode_url, order, title: str):
    file_path = os.path.join(NOVEL_PATH, title)
    async with aiohttp.ClientSession() as session:
        async with session.get(episode_url, cookies=cookies) as res:
            result = await res.text()
    soup = BeautifulSoup(result, "lxml")
    episode_title = soup.find(name="div", class_="col-xl-9 col-lg-8 p-r-30").find(name="h2").text
    content_block = soup.find(name="div", class_="forum-content mt-3")
    pic_tag = soup.find(name="div", class_="forum-content mt-3").find_all(name="img")
    pic_url_list = set()
    for p in pic_tag:
        pic_url_list.add(p.get("src"))
    illustration_url_list.append({title: pic_url_list})

    novel_episode_mp[title + str(order)] = str(episode_title)
    tmp_nme = title + str(order)
    logging.info("开始写入: 《" + tmp_nme + "》\n")
    file = open(os.path.join(file_path, "OEBPS", "Text", str(order) + ".xhtml"), "w", encoding="utf-8")
    file.write(create_episode(str(episode_title), str(content_block)))


async def fetch_episode_art(title_illustration_url_list, title):
    tasks = []
    for illustration_url in title_illustration_url_list[title]:
        name = str(hashlib.md5(illustration_url.encode(encoding="UTF-8")).hexdigest()) + ".jpg"
        illustration_url_mp[illustration_url] = name
        task = asyncio.create_task(download_illustration(illustration_url, title, name))
        tasks.append(task)
    await asyncio.wait(tasks, timeout=None)


async def download_illustration(illustration_url, title, name):
    logging.info(f"开始保存《{title}》图片: {illustration_url}")
    async with semaphore:
        try:
            async with aiohttp.ClientSession(trust_env=True) as session:
                async with session.get(illustration_url, cookies=cookies) as res:
                    result = await res.read()
            pic_path = os.path.join(NOVEL_PATH, title, "OEBPS", "Images")
            with open(os.path.join(pic_path, name), "wb") as pic:
                pic.write(result)
            logging.info(f"保存《{title}》图片: {illustration_url}成功")
        except ConnectionResetError:
            logging.warning(f"保存《{title}》图片: {illustration_url}失败， 因为远程主机强迫关闭了一个现有的连接")
            await asyncio.sleep(0)
    


def create_title_page(title_dir):
    content = ("<?xml version=\"1.0\" encoding=\"utf-8\"?><!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.1//EN\"  "
               "\"http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd\"><html xmlns=\"http://www.w3.org/1999/xhtml\" "
               "xml:lang=\"zh-CN\"><head><title>封面</title></head><body><div><img "
               "src=\"../Images/cover.jpg\"/></div></body></html>")
    with open(os.path.join(title_dir, "OEBPS", "Text", "titlepage.xhtml"), "w", encoding="utf-8") as tp:
        tp.write(content)


def create_toc(title, author):
    content = ("<?xml version=\"1.0\" encoding=\"utf-8\" ?><!DOCTYPE ncx PUBLIC \"-//NISO//DTD ncx 2005-1//EN\" "
               "\"http://www.daisy.org/z3986/2005/ncx-2005-1.dtd\"><ncx version=\"2005-1\" "
               "xmlns=\"http://www.daisy.org/z3986/2005/ncx/\"><head><meta "
               "content=\"urn:uuid:5208e6bb-5d25-45b0-a7fd-b97d79a85fd4\" name=\"dtb:uid\"/><meta content=\"0\" "
               "name=\"dtb:depth\"/><meta content=\"0\" name=\"dtb:totalPageCount\"/><meta content=\"0\" "
               "name=\"dtb:maxPageNumber\"/></head><docTitle><text>")
    content += title
    content += "</text></docTitle><docAuthor><text>"
    content += author
    content += ("</text></docAuthor><navMap><navPoint id=\"cover\" "
                "playOrder=\"0\"><navLabel><text>封面</text></navLabel><content "
                "src=\"Text/titlepage.xhtml\"/></navPoint>")

    for idx in range(1, len(os.listdir(os.path.join(NOVEL_PATH, title, "OEBPS", "Text"))) - 1):
        content += "<navPoint id=\"ep"
        content += str(idx)
        content += "\" playOrder=\""
        content += str(idx)
        content += "\"><navLabel><text>"
        content += novel_episode_mp[title + str(idx - 1)]
        content += "</text></navLabel><content src=\"Text/"
        content += str(idx) + ".xhtml\"/></navPoint>"
    content += "</navMap></ncx>"
    with open(os.path.join(NOVEL_PATH, title, "OEBPS", "toc.ncx"), "w", encoding="utf-8") as toc:
        toc.write(content)


def create_episode(episode_title, content_block):
    content = ("<?xml version=\"1.0\" encoding=\"utf-8\"?><!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.1//EN\"  "
               "\"http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd\"><html xmlns=\"http://www.w3.org/1999/xhtml\" "
               "xmlns:xml=\"http://www.w3.org/XML/1998/namespace\" xml:lang=\"zh-CN\"><head><title>")
    content += episode_title
    content += "</title></head><body>"
    content += "<h1>" + episode_title + "</h1>"
    content += content_block
    content += "</body></html>"
    return content


# TODO: 之前写这玩意没想到用EbookLib库，多少有点亏，手动照结构创建还是太麻烦了，重构封装一下用EbookLib造epub得了

if __name__ == '__main__':
    start = time.time()
    asyncio.run(gen_multi_novels(Config.esj_novel_url()))
    end = time.time()
    logging.info(f"执行时间:{end - start}")
