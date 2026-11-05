"""中文开发术语 → 英文关键词映射。

这是整个产品的核心资产。Phase 0 要验证的就是：这张表够不够用。

结构：中文概念 → 英文同义词族（任一命中即可）
匹配：最长优先，避免「文件」吃掉「文件系统」
"""

CONCEPTS = {
    # ---------- 文件操作 ----------
    "批量重命名": ["batch rename", "bulk rename", "renamer", "mass rename"],
    "重命名": ["rename", "renamer", "mv"],
    "批量": ["batch", "bulk", "mass"],
    "文件管理": ["file manager", "file-manager", "filebrowser"],
    "文件同步": ["file sync", "syncthing", "rsync", "sync"],
    "文件传输": ["file transfer", "file sharing", "file-sharing"],
    "文件系统": ["filesystem", "file system"],
    "备份": ["backup"],
    "恢复": ["restore", "recovery"],
    "解压": ["unzip", "extract", "decompress"],
    "压缩": ["compress", "zip", "archive", "compression"],
    "拖拽": ["drag and drop", "drag-and-drop", "dnd"],
    "去重": ["deduplicate", "dedup", "duplicate finder"],

    # ---------- 文本 ----------
    "文本编辑": ["text editor"],
    "编辑器": ["editor"],
    "笔记": ["note", "notes", "note-taking"],
    "知识库": ["knowledge base", "knowledge-base", "wiki"],
    "文档": ["document", "docs", "documentation"],
    "博客": ["blog"],
    "搜索": ["search", "find", "lookup"],
    "全文搜索": ["full text search", "full-text-search", "fts"],
    "正则": ["regex", "regular expression", "regexp"],
    "格式化": ["formatter", "formatting", "pretty print"],
    "语法高亮": ["syntax highlighting", "syntax-highlighting", "highlight"],
    "翻译": ["translate", "translation", "translator"],
    "拼写检查": ["spell check", "spellcheck", "spelling"],

    # ---------- 网络 ----------
    "爬虫": ["crawler", "scraper", "spider", "crawl", "scrape"],
    "抓取": ["scrape", "fetch", "crawl"],
    "下载": ["download", "downloader"],
    "上传": ["upload", "uploader"],
    "代理": ["proxy"],
    "内网穿透": ["tunnel", "nat traversal", "frp", "ngrok"],
    "网页": ["web page", "webpage", "html"],
    "网站": ["website", "web app", "webapp"],
    "接口": ["api", "rest api", "rest-api"],
    "服务端": ["server", "backend"],
    "前端": ["frontend", "front-end", "ui"],
    "后端": ["backend", "back-end"],
    "全栈": ["fullstack", "full-stack"],
    "浏览器": ["browser"],
    "书签": ["bookmark", "bookmarks"],

    # ---------- 界面 ----------
    "界面": ["ui", "interface", "gui"],
    "管理面板": ["admin panel", "admin dashboard", "admin-panel"],
    "仪表盘": ["dashboard"],
    "图表": ["chart", "graph", "plot", "visualization"],
    "可视化": ["visualization", "visualize", "dataviz"],
    "表格": ["table", "spreadsheet", "datagrid", "data grid"],
    "主题": ["theme"],
    "插件": ["plugin", "extension"],
    "组件": ["component", "widget"],
    "动画": ["animation", "animate"],

    # ---------- 数据 ----------
    "数据库": ["database", "db"],
    "缓存": ["cache", "caching"],
    "队列": ["queue", "message queue", "message-queue"],
    "消息": ["message", "messaging"],
    "日志": ["log", "logging"],
    "监控": ["monitoring", "monitor", "observability"],
    "指标": ["metrics"],
    "数据集": ["dataset"],
    "数据迁移": ["migration", "migrate"],
    "数据": ["data"],

    # ---------- AI ----------
    "机器学习": ["machine learning", "machine-learning", "ml"],
    "深度学习": ["deep learning", "deep-learning"],
    "神经网络": ["neural network", "neural-network"],
    "大模型": ["llm", "large language model", "gpt"],
    "自然语言": ["nlp", "natural language", "natural-language"],
    "图像识别": ["image recognition", "image classification"],
    "目标检测": ["object detection", "object-detection"],
    "语音识别": ["speech recognition", "asr"],
    "语音合成": ["text to speech", "tts", "speech synthesis"],
    "文字识别": ["ocr", "text recognition"],
    "训练": ["training", "train"],
    "推理": ["inference"],
    "微调": ["fine-tuning", "finetune"],
    "提示词": ["prompt", "prompt engineering"],
    "向量": ["vector", "embedding"],
    "检索增强": ["rag", "retrieval augmented generation"],
    "智能体": ["agent", "agents"],

    # ---------- 自动化 ----------
    "自动化": ["automation", "automate"],
    "脚本": ["script", "scripting"],
    # 2026-11-05 实测：`cli json` 返回 httpie、urfave/cli 这类叫 cli 的仓库；
    # `command line json` 才是 jq / yq。GitHub 会拿词去撞仓库名，缩写是重灾区。
    "命令行": ["command line", "cli", "command-line"],
    "终端": ["terminal", "tui"],
    "快捷键": ["hotkey", "shortcut", "keybinding"],
    "定时": ["scheduler", "cron", "scheduled"],
    "工作流": ["workflow", "pipeline"],
    "任务": ["task", "job"],

    # ---------- 开发 ----------
    "脚手架": ["boilerplate", "scaffold", "starter", "template"],
    "模板": ["template", "boilerplate"],
    "框架": ["framework"],
    "库": ["library", "lib"],
    "构建": ["build", "bundler"],
    "打包": ["bundler", "packaging", "packager"],
    "编译": ["compiler", "compile"],
    "调试": ["debug", "debugger"],
    "测试": ["test", "testing"],
    "性能": ["performance", "benchmark"],
    "部署": ["deploy", "deployment"],
    "容器": ["container", "docker"],
    "编排": ["orchestration", "kubernetes"],
    "版本控制": ["version control", "git"],
    "代码审查": ["code review", "code-review"],
    "文档生成": ["documentation generator", "docs generator"],

    # ---------- 安全 ----------
    "加密": ["encryption", "encrypt", "crypto", "cryptography"],
    "解密": ["decrypt", "decryption"],
    "密码": ["password"],
    "密码管理": ["password manager", "password-manager"],
    "密钥": ["key", "secret", "keys"],
    "认证": ["authentication", "auth"],
    "授权": ["authorization", "oauth"],
    "权限": ["permission", "rbac"],
    "漏洞": ["vulnerability", "cve"],
    "扫描": ["scanner", "scan"],
    "安全": ["security"],

    # ---------- 媒体 ----------
    "图片": ["image", "picture", "photo"],
    "图片处理": ["image processing", "image manipulation", "image-processing"],
    "视频": ["video"],
    "视频处理": ["video processing", "video-processing"],
    "音频": ["audio", "sound"],
    "剪辑": ["editor", "cutting"],
    "截图": ["screenshot", "capture"],
    "录屏": ["screen recording", "screen recorder", "screen-recorder"],
    "字幕": ["subtitle", "subtitles", "srt"],
    "转码": ["transcode", "transcoding", "ffmpeg"],
    "播放器": ["player"],
    "阅读器": ["reader"],

    # ---------- 平台 ----------
    # 桌面软件几乎不在描述里写 "desktop"，而是列操作系统
    "桌面": ["desktop", "windows", "macos", "linux"],
    "移动端": ["mobile"],
    "安卓": ["android"],
    "苹果": ["ios", "macos"],
    "跨平台": ["cross-platform", "cross platform"],
    "系统": ["system", "os"],
    "进程": ["process"],
    "磁盘": ["disk"],
    "游戏": ["game"],
    "引擎": ["engine"],
    "地图": ["map", "maps"],
    "天气": ["weather"],
    "股票": ["stock", "finance"],
    "音乐": ["music"],

    # ---------- 效率 ----------
    "待办": ["todo", "task list", "todo-list"],
    "日历": ["calendar"],
    "提醒": ["reminder", "notification"],
    "通知": ["notification", "notify"],
    "邮件": ["email", "mail"],
    "聊天": ["chat", "messaging"],
    "机器人": ["bot"],
}

# 中文填充词，匹配时直接丢弃（不产生查询词）
FILLER = {
    "的", "能", "可以", "一个", "帮我", "找", "找到", "想要", "需要",
    "有没有", "什么", "好用", "开源", "项目", "软件", "程序", "应用",
    "支持", "用于", "实现", "功能", "东西", "推荐", "那种", "这种",
    "我", "要", "想", "个", "了", "和", "跟", "或者", "还是", "比较",
    "最好", "一些", "些", "用来", "能够", "工具", "小工具", "跑", "做",
    "把", "被", "给", "在", "上", "下", "里", "中", "很", "好", "好看",
}

# ---------- 第二轮补充：跑通基础查询后按缺口补的 ----------
CONCEPTS.update({
    # 文件
    "文件": ["file", "files"],
    "文件夹": ["folder", "directory", "dir"],
    "目录": ["directory", "dir"],
    "转换": ["convert", "conversion", "converter"],
    "格式转换": ["format conversion", "converter", "convert"],
    "合并": ["merge", "combine", "concat"],
    "拆分": ["split", "separate"],
    "导入": ["import", "importer"],
    "导出": ["export", "exporter"],
    "查找替换": ["find and replace", "search and replace", "replace"],
    "替换": ["replace", "replacer"],
    "对比": ["diff", "compare", "comparison"],
    "差异": ["diff", "difference"],
    "预览": ["preview"],
    "生成": ["generate", "generator"],
    "解析": ["parser", "parse"],
    "提取": ["extract", "extraction"],
    "安装": ["install", "installer"],
    "卸载": ["uninstall"],
    "复制": ["copy", "clone"],
    "粘贴": ["paste"],
    "剪贴板": ["clipboard"],
    "历史": ["history"],
    "记录": ["record", "history"],
    "统计": ["statistics", "stats"],
    "报表": ["report", "reporting"],
    "清理": ["cleaner", "cleanup", "clean"],
    "优化": ["optimize", "optimization"],

    # 部署形态
    "本地": ["local", "offline"],
    "转成": ["convert", "conversion", "convert to"],
    "转": ["convert", "convert to"],
    "自建": ["self-hosted", "selfhosted", "self hosted"],
    "搭建": ["build", "deploy", "setup", "host"],
    "自己搭": ["self-hosted", "selfhosted", "self hosted"],
    "离线": ["offline", "local"],
    "在线": ["online", "web"],
    "自托管": ["self-hosted", "selfhosted"],
    "轻量": ["lightweight", "tiny"],
    "快速": ["fast", "quick", "blazing"],
    "多语言": ["i18n", "internationalization", "multilingual"],

    # 系统 / 硬件
    "内存": ["memory"],
    "硬件": ["hardware"],
    "驱动": ["driver", "drivers"],
    "设备": ["device"],
    "串口": ["serial port", "serial"],
    "蓝牙": ["bluetooth", "ble"],
    "摄像头": ["camera", "webcam"],
    "麦克风": ["microphone", "mic", "audio input"],
    "传感器": ["sensor", "sensors"],
    "无人机": ["drone"],
    "导航": ["navigation", "navigate"],
    "定位": ["location", "geolocation", "gps"],
    "壁纸": ["wallpaper"],
    "托盘": ["system tray", "tray"],
    "启动器": ["launcher", "launch"],
    "菜单": ["menu"],

    # 网络 / 运维
    "服务器": ["server"],
    "客户端": ["client"],
    "协议": ["protocol"],
    "网络": ["network", "networking"],
    "测速": ["speedtest", "bandwidth"],
    "加速": ["accelerator", "boost", "speed up"],
    "压测": ["stress test", "load test", "load-test"],
    "基准": ["benchmark", "benchmarking"],
    "反爬": ["anti-scraping", "anti-bot", "bypass"],
    "代理池": ["proxy pool", "proxy-pool"],
    "验证码": ["captcha"],

    # 商业 / 内容
    "商城": ["shop", "store", "ecommerce", "e-commerce"],
    "电商": ["ecommerce", "e-commerce"],
    "支付": ["payment", "pay"],
    "账单": ["billing", "invoice"],
})

# ---------- 第三轮补充：跑完 20 条评测后按实际失败案例补的 ----------
# 判据：这条查询在 reports/eval.md 里明确答错，且缺的是词不是候选池。
CONCEPTS.update({
    "局域网": ["lan", "local network", "local-network", "local area network"],
    "传文件": ["file transfer", "file sharing", "send file"],
    "识别": ["recognition", "recognize", "ocr", "detect"],
    "文字": ["text", "ocr"],
    "代码补全": ["code completion", "code-completion", "autocomplete", "intellisense", "copilot"],
    "补全": ["completion", "autocomplete"],
    "保存": ["save", "export"],
    "下载器": ["downloader", "download"],   # 实测 command line downloader 能捞出 yt-dlp ★205k
    "电子书": ["ebook", "e-book"],
})

# 这些是「意图修饰」而非检索概念，留下反而把结果带偏
FILLER |= {
    "查看", "自动", "状态", "器", "用", "写", "成",
    "中文",   # 「支持中文的X」里它是修饰语；当成概念会捞出所有 Chinese-* 项目
    "手机",   # 实测 android phone ssh client 只有 17 条垃圾，去掉 phone 有 277 条
}

# ---------- 概念组合塌缩 ----------
# 中文口语习惯把两三个词并排说出来，但英文生态里它们是一个专门词。
# 逐词翻译会造出 `recognition image text offline`（29 条垃圾），
# 正确说法是 `ocr offline`（2582 条，Umi-OCR ★48k 第一）。
#
# (触发键集合, 一并吞掉的键, 替换成的新组 or None)
COMBOS = [
    ({"识别", "文字"}, {"识别", "图片", "文字"}, ["ocr"]),
    ({"识别", "图片"}, {"识别", "图片", "文字"}, ["ocr"]),
    ({"epub", "mobi"}, {"epub", "mobi", "azw3", "转成", "转换", "转", "格式转换"},
     ["ebook converter"]),
    ({"epub", "azw3"}, {"epub", "mobi", "azw3", "转成", "转换", "转", "格式转换"},
     ["ebook converter"]),
    ({"mobi", "azw3"}, {"epub", "mobi", "azw3", "转成", "转换", "转", "格式转换"},
     ["ebook converter"]),
    # 冗余限定：「终端里的文本编辑器」= terminal editor，多一个 text 就是第四个 AND 条件
    ({"文本编辑", "终端"}, {"文本编辑"}, ["editor"]),
    # 补全本来就发生在编辑器里，editor 是容器不是功能
    ({"代码补全", "编辑器"}, {"编辑器"}, None),
]

# 组数 ≥3 时才丢：这些词是平台/包装/泛化动词，GitHub 描述里要么不写，要么人人都有。
# 数量少时不丢——「开源的视频剪辑软件」只剩 video + editor，editor 就是本体。
WEAK_CONCEPTS = {"桌面", "客户端", "保存", "生成"}

# 最长优先匹配用的键列表
_KEYS_BY_LEN = sorted(CONCEPTS.keys(), key=len, reverse=True)
_FILLER_BY_LEN = sorted(FILLER, key=len, reverse=True)
