"""候选池采集用的 GitHub topics。

原则：覆盖要宽。Phase 0 测的是「匹配算法」，不是「池子够不够大」。
池子里没有正确答案，任何算法都会失败。
"""

TOPICS = [
    # AI / ML
    "machine-learning", "deep-learning", "neural-network", "llm",
    "artificial-intelligence", "nlp", "computer-vision",
    "reinforcement-learning", "pytorch", "tensorflow", "transformers",
    "llm-inference", "rag", "agent", "embedding",

    # 语言 / 运行时
    "python", "rust", "golang", "java", "kotlin", "swift",
    "typescript", "javascript", "cpp", "csharp", "lua", "zig",

    # Web 前端
    "react", "vue", "angular", "svelte", "nextjs", "nuxt",
    "tailwindcss", "css", "web-components", "frontend",

    # Web 后端
    "nodejs", "deno", "bun", "django", "flask", "fastapi",
    "spring-boot", "rails", "laravel", "backend", "graphql", "rest-api",

    # CLI / 终端
    "cli", "command-line", "terminal", "tui", "shell", "bash",
    "zsh", "powershell",

    # 数据库 / 存储
    "database", "sql", "postgresql", "mysql", "sqlite", "mongodb",
    "redis", "vector-database", "key-value-store", "orm",

    # DevOps / 基础设施
    "devops", "docker", "kubernetes", "ci-cd", "terraform", "ansible",
    "infrastructure-as-code", "self-hosted", "homelab", "monitoring",

    # 安全
    "security", "cryptography", "authentication", "penetration-testing",
    "password-manager", "vulnerability-scanner",

    # 数据 / 科学计算
    "data-science", "data-visualization", "data-analysis", "pandas",
    "jupyter", "scientific-computing", "simulation", "math",

    # 媒体处理
    "image-processing", "video", "audio", "ffmpeg", "ocr",
    "speech-recognition", "text-to-speech", "pdf", "subtitle",

    # 桌面 / 移动
    "electron", "tauri", "qt", "gtk", "flutter", "react-native",
    "android", "ios", "desktop-app",

    # 编辑器 / IDE
    "editor", "vim", "neovim", "vscode-extension", "language-server",

    # 效率 / 知识管理
    "productivity", "note-taking", "markdown", "knowledge-base",
    "wiki", "bookmark-manager", "rss",

    # 自动化 / 爬虫
    "automation", "workflow", "scraping", "crawler", "browser-automation",
    "scheduler", "cron",

    # 文件 / 系统
    "file-manager", "file-sharing", "backup", "sync", "archive",
    "compression", "filesystem",

    # 网络
    "http", "websocket", "proxy", "vpn", "dns", "network",

    # 通信 / 机器人
    "bot", "chat", "email", "notification", "discord", "telegram",
    "wechat", "slack",

    # 游戏 / 图形
    "game-engine", "game-development", "graphics", "opengl", "vulkan",
    "webgl", "raytracing",

    # 嵌入式 / IoT
    "iot", "embedded", "arduino", "esp32", "raspberry-pi", "robotics",

    # 工具链
    "compiler", "interpreter", "parser", "regex", "build-tool",
    "package-manager", "linter", "formatter",

    # 其他
    "testing", "benchmark", "serialization", "caching", "message-queue",
    "streaming", "blockchain", "web3", "browser", "spreadsheet",
]

assert len(TOPICS) == len(set(TOPICS)), "topics 有重复"
