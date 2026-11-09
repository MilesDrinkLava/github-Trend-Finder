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

# ---------- 通用术语与完整词组 ----------
# 第一项用于远程查询，其余项供本地召回。平台、实现语言不是同义词。
CONCEPTS.update({
    "正则表达式": ["regex", "regular expression", "regexp"],
    "文本编辑器": ["text editor", "editor"],
    "代码编辑器": ["code editor", "editor"],
    "文件管理器": ["file manager", "file-manager", "filebrowser"],
    "密码管理器": ["password manager", "password-manager"],
    "中文": ["chinese", "zh-cn", "chinese language"],
    "汉化": ["chinese", "localization"],
    "国际化": ["internationalization", "i18n"],
    "本地化": ["localization", "l10n"],
    "简体中文": ["simplified chinese", "zh-cn"],
    "繁体中文": ["traditional chinese", "zh-tw"],
    "浏览器扩展": ["browser extension", "webextension"],
    "浏览器插件": ["browser extension", "browser plugin"],
    "浏览器自动化": ["browser automation", "headless browser"],
    "无头浏览器": ["headless browser"],
    "网页抓取": ["web scraping", "scraper", "crawler"],
    "网页截图": ["website screenshot", "webpage capture"],
    "网页转图片": ["html to image", "html screenshot"],
    "网页转pdf": ["html to pdf", "pdf generator"],
    "向量数据库": ["vector database", "vector store"],
    "关系型数据库": ["relational database", "sql database"],
    "时序数据库": ["time series database", "timeseries database"],
    "图数据库": ["graph database"],
    "数据库管理": ["database management", "database client"],
    "数据库迁移": ["database migration", "schema migration"],
    "数据库备份": ["database backup"],
    "数据可视化": ["data visualization", "dataviz"],
    "数据清洗": ["data cleaning", "data wrangling"],
    "数据标注": ["data annotation", "labeling"],
    "数据分析": ["data analysis", "analytics"],
    "数据采集": ["data collection", "data ingestion"],
    "分布式": ["distributed"],
    "消息队列": ["message queue", "message broker"],
    "全文检索": ["full text search", "fts"],
    "模糊匹配": ["fuzzy matching", "approximate matching"],
    "模糊搜索": ["fuzzy search"],
    "语义搜索": ["semantic search", "vector search"],
    "拼写纠错": ["spell correction", "spellcheck"],
    "自动补全": ["autocomplete", "completion"],
    "图片识别": ["image recognition", "image classification"],
    "图像分类": ["image classification"],
    "物体识别": ["object detection", "image recognition"],
    "物体": ["object", "objects"],
    "人脸识别": ["face recognition", "facial recognition"],
    "图片文字识别": ["ocr", "text recognition"],
    "图片压缩": ["image compression", "image optimizer"],
    "图片去重": ["duplicate image finder", "image deduplication"],
    "图片放大": ["image upscaling", "super resolution"],
    "超分辨率": ["super resolution", "image upscaling"],
    "背景移除": ["background removal", "remove background"],
    "抠图": ["background removal", "image matting"],
    "语音转文字": ["speech to text", "asr"],
    "文字转语音": ["text to speech", "tts"],
    "声音克隆": ["voice cloning"],
    "字幕生成": ["subtitle generation", "transcription"],
    "视频下载": ["video downloader", "video download"],
    "视频剪辑": ["video editor", "video editing"],
    "视频压缩": ["video compression", "video encoding"],
    "音频转换": ["audio converter", "audio conversion"],
    "屏幕录制": ["screen recorder", "screen recording"],
    "文档转换": ["document converter", "document conversion"],
    "文档解析": ["document parser", "document parsing"],
    "文档问答": ["document question answering", "rag"],
    "扫描件": ["scanned document", "scan"],
    "发票识别": ["invoice ocr", "invoice extraction"],
    "表格识别": ["table extraction", "table recognition"],
    "思维导图": ["mind map", "mindmap"],
    "流程图": ["flowchart", "diagram"],
    "白板": ["whiteboard"],
    "看板": ["kanban", "board"],
    "项目管理": ["project management"],
    "时间追踪": ["time tracking", "time tracker"],
    "番茄钟": ["pomodoro", "pomodoro timer"],
    "剪贴板管理": ["clipboard manager", "clipboard history"],
    "远程桌面": ["remote desktop", "rdp"],
    "远程控制": ["remote control", "remote access"],
    "屏幕共享": ["screen sharing"],
    "文件分享": ["file sharing", "file transfer"],
    "断点续传": ["resumable download", "resume download"],
    "端口转发": ["port forwarding", "port forward"],
    "反向代理": ["reverse proxy"],
    "负载均衡": ["load balancer", "load balancing"],
    "服务发现": ["service discovery"],
    "服务网格": ["service mesh"],
    "证书管理": ["certificate management", "tls certificate"],
    "单点登录": ["single sign on", "sso"],
    "双因素认证": ["two factor authentication", "2fa"],
    "访问控制": ["access control", "rbac"],
    "密码生成": ["password generator"],
    "密钥管理": ["secret management", "key management"],
    "漏洞扫描": ["vulnerability scanner", "security scanner"],
    "依赖管理": ["dependency management", "package manager"],
    "包管理器": ["package manager"],
    "静态分析": ["static analysis", "linter"],
    "单元测试": ["unit testing", "unit test"],
    "端到端测试": ["end to end testing", "e2e testing"],
    "持续集成": ["continuous integration", "ci"],
    "持续部署": ["continuous deployment", "cd"],
    "代码生成": ["code generation", "codegen"],
    "接口测试": ["api testing", "api client"],
    "接口文档": ["api documentation", "openapi"],
    "接口网关": ["api gateway"],
    "低代码": ["low code", "low-code"],
    "无代码": ["no code", "no-code"],
    "静态网站": ["static site", "static website"],
    "内容管理": ["content management", "cms"],
    "无障碍": ["accessibility", "a11y"],
    "响应式": ["responsive"],
    "虚拟列表": ["virtual list", "virtualization"],
    "富文本编辑器": ["rich text editor", "wysiwyg"],
    "代码高亮": ["syntax highlighting"],
    "嵌入模型": ["embedding model", "text embedding"],
    "重排序": ["reranker", "reranking"],
    "检索增强生成": ["rag", "retrieval augmented generation"],
    "本地大模型": ["local llm", "llm inference"],
    "模型量化": ["model quantization", "quantization"],
    "模型部署": ["model serving", "model deployment"],
    "多模态": ["multimodal"],
    "函数调用": ["function calling", "tool calling"],
    "智能体框架": ["agent framework", "ai agents"],
    "人工智能": ["artificial intelligence", "ai"],
    "增强现实": ["augmented reality", "ar"],
    "虚拟现实": ["virtual reality", "vr"],
    "嵌入式": ["embedded"],
    "单片机": ["microcontroller", "mcu"],
    "树莓派": ["raspberry pi"],
    "物联网": ["iot", "internet of things"],
    "三维": ["3d"],
    "二维": ["2d"],
    "三维建模": ["3d modeling", "3d modeller"],
    "二维码": ["qr code", "qrcode"],
    "条形码": ["barcode"],
    "识别": ["recognition", "recognize", "detect"],
    "文字": ["text"],
    "桌面": ["desktop"],
    "本地": ["local"],
    "离线": ["offline"],
    "苹果": ["apple"],
})

# 常见口语/后缀直接归到同一概念，不让「器」「程式」变成未知片段。
CONCEPT_ALIASES = {
    "文本编辑器": "文本编辑", "密码管理器": "密码管理", "文件管理器": "文件管理",
    "批量改名": "批量重命名", "批量改文件名": "批量重命名",
    "改文件名": "重命名", "改名": "重命名", "资料库": "数据库",
    "程式编辑器": "代码编辑器", "文字编辑器": "文本编辑器",
    "档案管理器": "文件管理器", "档案同步": "文件同步",
    "撷取": "抓取", "萤幕录制": "屏幕录制", "萤幕截图": "截图",
    "文字辨识": "文字识别", "图像辨识": "图像识别",
    "语音辨识": "语音识别", "排程": "定时", "设定": "配置",
    "自部署": "自托管",
    "档案": "文件", "萤幕": "屏幕",
}
CONCEPTS["配置"] = ["configuration", "config"]
CONCEPTS["屏幕"] = ["screen"]
for _alias, _target in CONCEPT_ALIASES.items():
    CONCEPTS[_alias] = list(CONCEPTS[_target])

# ASCII 技术名保留符号后再归一化；不把 js 与 JSON、C 与 C++ 混为一词。
ASCII_ALIASES = {
    "c++": "cpp", "c#": "csharp", "f#": "fsharp", ".net": "dotnet",
    "js": "javascript", "ts": "typescript", "py": "python",
    "golang": "go", "k8s": "kubernetes", "nodejs": "node.js",
    "vuejs": "vue", "reactjs": "react", "postgres": "postgresql",
}

# 确定的一对一常用字形映射，不做有歧义的通用繁简转换（例如「乾」「著」）。
CHAR_MAP = dict(zip(
    "軟體檔案夾網頁瀏覽擴寫載壓縮圖識別語轉換碼庫數據開發編輯搜尋簡單終端執訓練聲畫處動態視訊螢幕遠連線離時鐘雲務臺灣與為這個裡後顯錄製備復傳輸鑰權證訂閱類組織彙總統計設儲隱護刪除掃描資優選擇標籤應關閉輕級內驗務註釋產現無障礙雙啟點擊頁訊實際驗證間歸檢從讓獲願參庫別認劃務區塊鏈頁聲訊訂託簡轉",
    "软体档案夹网页浏览扩写载压缩图识别语转换码库数据开发编辑搜寻简单终端执训练声画处动态视讯萤幕远连线离时钟云务台湾与为这个里后显录制备复传输钥权证订阅类组织汇总统计设储隐护删除扫描资优选择标签应关闭轻级内验务注释产现无障碍双启点击页讯实际验证间归检从让获愿参库别认划务区块链页声讯订托简转",
))
CHAR_MAP.update({
    "詞": "词", "鍵": "键", "電": "电", "郵": "邮", "帳": "账",
    "號": "号", "機": "机", "腦": "脑", "愛": "爱", "讀": "读",
    "筆": "笔", "記": "记", "曆": "历", "歷": "历", "務": "务",
    "換": "换", "對": "对", "話": "话", "聽": "听", "訊": "讯",
    "體": "体", "碼": "码", "檢": "检", "測": "测", "試": "试",
    "錯": "错", "誤": "误", "補": "补", "全": "全", "運": "运",
    "維": "维", "護": "护", "連": "连", "結": "结",
    "構": "构", "啟": "启", "關": "关", "閉": "闭", "壞": "坏",
})
FILLER |= {"程式", "应用程式", "中的", "里面", "一下", "一款", "一套", "进行"}
FILLER.add("软体")
FILLER -= set(CONCEPTS)
COMBOS.insert(0, ({"识别", "物体"}, {"识别", "图片", "物体"}, ["object detection"]))

# 最长优先匹配用的键列表
_KEYS_BY_LEN = sorted(CONCEPTS.keys(), key=len, reverse=True)
_FILLER_BY_LEN = sorted(FILLER, key=len, reverse=True)
