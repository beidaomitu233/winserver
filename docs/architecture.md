一、推荐的最终技术栈

针对你要开发的“新一代 phpStudy”，我最推荐：

Vue 3 + TypeScript + Tauri 2 + Rust Core Agent + SQLite + Windows Service

不是把所有能力都写进桌面界面，而是拆成：

桌面界面
   ↓
Tauri 安全桥接层
   ↓
本地 Core Agent
   ↓
Nginx / Apache / PHP / MySQL / Redis

完整技术选型如下：

层级	推荐技术
UI	Vue 3 + TypeScript
构建工具	Vite
状态管理	Pinia
路由	Vue Router
样式	SCSS + CSS Variables
桌面容器	Tauri 2
核心后台	Rust
异步任务	Tokio
Windows API	windows Rust crate
UI 与后台通信	Windows Named Pipe + JSON-RPC
本地数据库	SQLite
配置模板	MiniJinja 或 Tera
日志	tracing + tracing-subscriber
HTTP 下载	reqwest
序列化	serde
错误处理	thiserror + anyhow
应用更新	Tauri Updater
组件更新	自研 Runtime Package Manager
前端测试	Vitest + Playwright
后端测试	cargo test + 集成测试

Vue 官方推荐在 TypeScript 项目中使用 Composition API，它的类型推导更完善；Pinia也是 Vue 当前推荐的状态管理方案。Tauri 2 可以使用任意 Web 前端框架，同时通过 Rust处理本地能力，并提供 capabilities 权限边界来限制前端能够调用的系统能力。

二、为什么推荐 Tauri，而不是 Electron

Electron 确实开发快，但这类软件并不是普通桌面客户端，它要处理：

启动和停止系统进程
获取管理员权限
修改 hosts
管理 Windows 服务
检测端口
管理进程树
生成服务器配置
下载并校验可执行程序
处理升级与回滚

因此，核心后台不应该由 JavaScript 直接控制。

Tauri 更适合做这种“现代 UI + 系统级后台”的软件：

Vue 负责展示和交互
Rust 负责真正的系统操作

Tauri 2 支持打包外部二进制程序，也支持通过 capabilities 对允许执行的程序、参数和窗口权限进行限制。它的 Updater 还支持静态或动态更新服务器，并要求更新包提供签名字段。

但是要注意：

不要因为 Tauri 有 shell 插件，就让 Vue 页面直接调用 nginx.exe、cmd.exe 或 PowerShell。

正确方式是：

Vue
 ↓ invoke
Tauri Rust Command
 ↓ Named Pipe
Core Agent
 ↓ 受控执行
nginx.exe / php-cgi.exe / mysqld.exe
三、整体架构设计

推荐拆成四个核心部分。

┌─────────────────────────────────────────┐
│             Desktop UI                  │
│ Vue 3 + TypeScript + Tauri 2            │
│                                         │
│ 首页 / 网站 / 数据库 / PHP / 日志 / 设置 │
└──────────────────┬──────────────────────┘
                   │ Tauri Command
                   │
┌──────────────────▼──────────────────────┐
│          Tauri Bridge                   │
│ 参数校验 / IPC 客户端 / 事件转发         │
└──────────────────┬──────────────────────┘
                   │ Named Pipe
                   │ JSON-RPC
┌──────────────────▼──────────────────────┐
│          WinServer Core Agent           │
│                                         │
│ ProcessManager       SiteManager        │
│ ServiceManager       RuntimeManager     │
│ PortManager          ConfigManager      │
│ HostsManager         UpdateManager      │
│ CertificateManager   DiagnoseManager    │
└───────┬─────────────┬───────────────┬───┘
        │             │               │
┌───────▼───┐  ┌──────▼─────┐  ┌──────▼──────┐
│ Web Server│  │ PHP Runtime│  │ Data Service│
│ Nginx     │  │ php-cgi    │  │ MySQL       │
│ Apache    │  │ Composer   │  │ Redis       │
└───────────┘  └────────────┘  └─────────────┘
四、为什么一定要有 Core Agent
1. 桌面 UI 不应该一直使用管理员权限

如果让整个桌面应用以管理员身份运行，会带来几个问题：

Vue/WebView 中的代码也继承高权限
打开的文件、链接和子进程可能继承高权限
每次启动都出现 UAC
UI 漏洞会直接变成系统权限漏洞
用户拖入文件、打开项目时安全边界模糊

更合理的方式是：

Desktop UI：普通用户权限
Core Agent：受控高权限

Windows 应用可以通过 manifest 声明执行级别，系统在需要管理员权限时执行相应的提升流程。

2. Agent 可以注册为 Windows Service

发布版本中，将 Agent 注册为 Windows 服务：

WinServerAgent.exe --service

开发环境中使用控制台模式：

WinServerAgent.exe --console

同一个程序支持两种模式：

fn main() {
    if has_argument("--service") {
        run_as_windows_service();
    } else {
        run_as_console();
    }
}

Windows Service Control Manager 统一保存服务配置并控制服务的启动、停止和运行状态；Windows 服务还可以在用户未登录时运行。

这样即使关闭桌面界面：

Nginx
PHP
MySQL
Redis

仍然可以继续运行。

五、UI 与 Agent 的通信方式

推荐使用：

Windows Named Pipe + JSON-RPC

例如管道名称：

\\.\pipe\WinServer.Core

Windows Named Pipe 支持一个服务端与多个客户端之间进行双向通信，并且可以通过 Windows 安全描述符限制哪些用户可以连接。

请求格式：

{
  "id": "request_01",
  "method": "service.start",
  "params": {
    "serviceId": "nginx-1.29"
  }
}

响应格式：

{
  "id": "request_01",
  "success": true,
  "data": {
    "status": "running",
    "pid": 5328,
    "ports": [80, 443]
  }
}

事件推送：

{
  "event": "service.statusChanged",
  "data": {
    "serviceId": "nginx-1.29",
    "oldStatus": "starting",
    "newStatus": "running"
  }
}

下载进度：

{
  "event": "runtime.installProgress",
  "data": {
    "runtimeId": "php-8.3.20",
    "progress": 72,
    "stage": "extracting"
  }
}

这里不建议直接在 Named Pipe 中传输大型文件，只传：

命令
状态
日志
进度
错误信息
六、核心设计原理：控制面与运行面分离

整个软件可以分为两层。

控制面

你开发的软件属于控制面：

创建网站
修改配置
切换 PHP
启动服务
停止服务
查看日志
安装组件
检查端口
执行诊断
运行面

第三方程序属于运行面：

nginx.exe
httpd.exe
php-cgi.exe
mysqld.exe
redis-server.exe

Core Agent 不应该侵入或修改这些程序源码，而是通过四种方式控制它们：

启动参数
配置文件
系统信号或控制命令
进程与端口检测

例如 Nginx：

生成配置
  ↓
nginx -t
  ↓
nginx -s reload

Nginx 在 reload 时会先检查配置；成功后启动新的 worker，让旧 worker 平滑退出；失败则继续使用旧配置。

Apache 同样提供配置检查和 graceful 重启机制。

七、后台核心模块设计
1. ProcessManager

负责普通进程：

nginx.exe
php-cgi.exe
redis-server.exe

核心接口：

pub trait ProcessManager {
    fn spawn(&self, spec: ProcessSpec) -> Result<ProcessInfo>;
    fn stop(&self, process_id: u32) -> Result<()>;
    fn kill(&self, process_id: u32) -> Result<()>;
    fn exists(&self, process_id: u32) -> bool;
    fn get_metrics(&self, process_id: u32) -> Result<ProcessMetrics>;
}

Rust 标准库的 std::process::Command 可以创建和控制子进程，并获取退出状态、标准输出和错误输出。

但是不能只使用 Command，还需要结合 Windows Job Object。

2. Windows Job Object

Nginx、Apache、PHP 可能产生多个子进程。

例如：

nginx master
├─ nginx worker
├─ nginx worker
└─ nginx worker

如果只结束主 PID，可能留下孤儿进程。

正确做法：

一个服务实例
   ↓
一个 Windows Job Object
   ↓
这个服务产生的所有子进程

Windows Job Object 可以把多个进程作为一个整体管理，包括统一终止整个进程组、设置资源限制和查询进程信息。

例如：

Job: runtime-nginx-default
├─ nginx master
├─ nginx worker 1
├─ nginx worker 2
└─ nginx worker 3

停止时：

优先执行 graceful stop
       ↓
等待超时
       ↓
终止整个 Job Object

不要一上来就使用：

taskkill /F /IM nginx.exe

因为这可能误杀其他软件启动的 Nginx。

3. ServiceManager

负责 Windows Service：

MySQL
Agent 自身
可选的 Apache
可选的 Redis

接口：

pub trait ServiceManager {
    fn install(&self, spec: ServiceSpec) -> Result<()>;
    fn uninstall(&self, service_name: &str) -> Result<()>;
    fn start(&self, service_name: &str) -> Result<()>;
    fn stop(&self, service_name: &str) -> Result<()>;
    fn status(&self, service_name: &str) -> Result<ServiceStatus>;
}

不建议依赖：

sc.exe
net start
net stop

作为唯一实现。

正式版本应通过 Windows Service API 操作，命令行工具可以作为诊断备用方案。

4. PortManager

负责：

检查端口占用
分配 PHP FastCGI 端口
查找端口对应 PID
检查 IPv4、IPv6
防止端口冲突

数据结构：

pub struct PortBinding {
    pub port: u16,
    pub protocol: Protocol,
    pub address: String,
    pub pid: Option<u32>,
    pub owner_type: PortOwnerType,
    pub owner_id: Option<String>,
}

不要看到 80 端口被占用就直接结束程序。

应该显示：

端口：80
状态：已占用
PID：5328
程序：C:\Program Files\IIS Express\iisexpress.exe
启动用户：BeiDao
建议：停止 IIS Express 或将站点修改为 8080
5. RuntimeManager

所有软件都应该被抽象为 Runtime：

Nginx Runtime
Apache Runtime
PHP Runtime
MySQL Runtime
Redis Runtime
Composer Runtime

数据模型：

pub struct RuntimeManifest {
    pub id: String,
    pub runtime_type: RuntimeType,
    pub version: String,
    pub architecture: Architecture,
    pub entrypoints: Vec<Entrypoint>,
    pub files: Vec<RuntimeFile>,
    pub dependencies: Vec<Dependency>,
    pub checksum: String,
}

PHP 示例：

{
  "id": "php-8.3.20-nts-x64",
  "type": "php",
  "version": "8.3.20",
  "architecture": "x64",
  "threadSafe": false,
  "entrypoints": {
    "cli": "php.exe",
    "fastcgi": "php-cgi.exe"
  },
  "config": {
    "template": "php.ini.template"
  },
  "healthChecks": [
    {
      "type": "command",
      "command": "php.exe",
      "args": ["-v"]
    }
  ]
}
6. SiteManager

负责网站完整生命周期：

pub trait SiteManager {
    fn create(&self, input: CreateSiteInput) -> Result<Site>;
    fn update(&self, site_id: &str, input: UpdateSiteInput) -> Result<Site>;
    fn delete(&self, site_id: &str) -> Result<()>;
    fn enable(&self, site_id: &str) -> Result<()>;
    fn disable(&self, site_id: &str) -> Result<()>;
    fn switch_php(&self, site_id: &str, runtime_id: &str) -> Result<()>;
}

SiteManager 本身不直接写字符串配置，而是调用：

SiteManager
   ↓
ConfigCompiler
   ↓
NginxAdapter / ApacheAdapter
7. ConfigCompiler

配置必须由强类型数据生成：

数据库模型
   ↓
标准化 SiteConfig
   ↓
服务器模板
   ↓
临时配置
   ↓
语法验证
   ↓
正式配置

统一模型：

pub struct SiteConfig {
    pub id: String,
    pub name: String,
    pub domains: Vec<String>,
    pub document_root: PathBuf,
    pub server_type: ServerType,
    pub http_port: u16,
    pub https_port: Option<u16>,
    pub php_runtime_id: Option<String>,
    pub rewrite_template: Option<String>,
    pub ssl: Option<SslConfig>,
}

Nginx 模板：

server {
    listen {{ http_port }};
    server_name {{ domains | join(" ") }};

    root "{{ document_root }}";
    index index.php index.html;

    location / {
        try_files $uri $uri/ /index.php?$query_string;
    }

    location ~ \.php$ {
        try_files $uri =404;
        include fastcgi_params;
        fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;
        fastcgi_pass {{ php_host }}:{{ php_port }};
    }
}

输入路径和域名必须在进入模板前完成校验，不能直接把用户输入拼接进配置。

八、服务适配器设计

不要在业务代码中到处写：

if service_type == "nginx" {
    // ...
} else if service_type == "apache" {
    // ...
}

应该使用适配器：

pub trait ServerAdapter {
    fn probe(&self, runtime: &Runtime) -> Result<ProbeResult>;

    fn generate_config(
        &self,
        site: &SiteConfig,
        context: &ConfigContext
    ) -> Result<String>;

    fn validate_config(
        &self,
        runtime: &Runtime,
        config_path: &Path
    ) -> Result<ValidationResult>;

    fn start(&self, runtime: &Runtime) -> Result<ProcessHandle>;

    fn stop(&self, instance: &ServiceInstance) -> Result<()>;

    fn reload(&self, instance: &ServiceInstance) -> Result<()>;

    fn health_check(&self, instance: &ServiceInstance)
        -> Result<HealthStatus>;
}

具体实现：

NginxAdapter
ApacheAdapter
PhpFastCgiAdapter
MySqlAdapter
RedisAdapter

以后增加 Caddy、MariaDB 时，只需要增加适配器，不需要重写整个管理器。

九、三个最重要的状态概念

每个服务不能只有：

运行
停止

应该区分：

installed
starting
running
degraded
stopping
stopped
failed
unknown

同时区分两类状态。

Desired State

用户希望服务处于什么状态：

desired_state = running
Actual State

系统实际检测到的状态：

actual_state = failed

例如：

{
  "serviceId": "nginx-default",
  "desiredState": "running",
  "actualState": "failed",
  "failure": {
    "code": "PORT_OCCUPIED",
    "message": "80 端口已被占用"
  }
}

这样桌面界面才能准确显示：

用户希望启动，但实际启动失败

而不是按钮点完以后强行显示绿色“运行中”。

十、服务状态检测原理

不能只检查 PID。

正确状态检测至少包含四层：

进程检查
  +
端口检查
  +
配置检查
  +
业务健康检查

以 Nginx 为例：

1. nginx master PID 是否存在
2. 80/443 端口是否由目标进程监听
3. nginx -t 是否通过
4. http://127.0.0.1:目标端口 是否返回响应

以 PHP 为例：

1. php-cgi.exe 是否存在
2. FastCGI 端口是否监听
3. PHP 进程是否属于当前运行时
4. 测试 PHP 页面能否成功返回

以 MySQL 为例：

1. mysqld.exe 是否存在
2. 3306 是否监听
3. 能否完成握手
4. 能否执行 SELECT 1

只有全部通过，才能显示：

running

部分通过则显示：

degraded
十一、PHP 多版本运行方案
1. 每个 PHP 版本分配一个 FastCGI 端口

例如：

PHP 7.4 → 127.0.0.1:9074
PHP 8.1 → 127.0.0.1:9081
PHP 8.2 → 127.0.0.1:9082
PHP 8.3 → 127.0.0.1:9083
PHP 8.4 → 127.0.0.1:9084

启动：

php-cgi.exe -b 127.0.0.1:9083

对应站点配置：

fastcgi_pass 127.0.0.1:9083;

切换 PHP：

确保 PHP 8.4 正常运行
       ↓
生成新站点配置
       ↓
执行 nginx -t
       ↓
替换正式配置
       ↓
nginx -s reload
       ↓
执行 PHP 健康检查
       ↓
提交数据库
2. PHP 进程池

MVP 可以每个 PHP 版本运行一个监听进程。

后续版本可以实现：

PHP 8.3 Pool
├─ php-cgi : 9181
├─ php-cgi : 9182
├─ php-cgi : 9183
└─ php-cgi : 9184

Nginx：

upstream php_83_pool {
    server 127.0.0.1:9181;
    server 127.0.0.1:9182;
    server 127.0.0.1:9183;
    server 127.0.0.1:9184;
}

但是本地开发环境并不需要一开始就实现复杂进程池，先把单版本单监听链路做稳定。

十二、创建网站的完整实现流程

创建网站必须是一个事务。

用户提交网站信息
       ↓
校验域名、目录、端口
       ↓
验证 Nginx 和 PHP 运行时
       ↓
确保 PHP FastCGI 已启动
       ↓
生成临时虚拟主机配置
       ↓
执行 nginx -t
       ↓
备份现有配置
       ↓
写入正式配置
       ↓
更新 hosts
       ↓
平滑重载 Nginx
       ↓
执行 HTTP 健康检查
       ↓
提交 SQLite 事务

任意环节失败：

恢复旧配置
恢复 hosts
恢复数据库
重新加载旧配置
记录操作日志

伪代码：

pub async fn create_site(input: CreateSiteInput) -> Result<Site> {
    validate_site_input(&input)?;

    let runtime = runtime_repo.get(&input.php_runtime_id)?;
    runtime_manager.ensure_ready(&runtime).await?;

    let draft_site = Site::from_input(input);

    let generated = config_compiler.generate(&draft_site)?;
    config_validator.validate(&generated).await?;

    let backup = config_store.create_backup()?;

    let result = async {
        config_store.apply(&generated)?;
        hosts_manager.add_domains(&draft_site.domains)?;
        server_manager.reload().await?;
        health_checker.check_site(&draft_site).await?;
        site_repo.insert(&draft_site)?;
        Ok(draft_site)
    }
    .await;

    if result.is_err() {
        config_store.restore(backup)?;
        hosts_manager.rollback()?;
        server_manager.reload().await?;
    }

    result
}
十三、组件安装与更新方案

应用更新和运行环境更新必须分开。

1. 应用自身更新

更新：

Desktop UI
Tauri Bridge
Core Agent

使用 Tauri Updater。

Tauri Updater 支持静态 JSON 或动态更新服务器，更新清单包含版本、下载地址和签名；Windows 还支持不同安装交互模式。

2. 运行环境更新

更新：

PHP
Nginx
Apache
MySQL
Redis
Composer

使用自研 Runtime Package Manager。

流程：

获取远程 manifest
       ↓
下载到 temp
       ↓
校验文件大小
       ↓
校验 SHA-256
       ↓
验证数字签名
       ↓
解压到 staging
       ↓
验证关键文件
       ↓
执行版本探测
       ↓
移动到正式版本目录
       ↓
登记到数据库

不要覆盖旧版本：

错误：
runtime/php/php83/

正确：
runtime/php/php-8.3.19/
runtime/php/php-8.3.20/

激活版本通过数据库或引用记录：

{
  "runtimeFamily": "php-8.3",
  "activeRuntimeId": "php-8.3.20-nts-x64"
}

这样更新失败可以直接切换回：

php-8.3.19
十四、文件目录设计

建议采用：

WinServer/
├─ app/
│  ├─ WinServer.exe
│  ├─ WinServerAgent.exe
│  └─ WebViewResources/
│
├─ runtimes/
│  ├─ nginx/
│  │  ├─ nginx-1.28.0/
│  │  └─ nginx-1.29.0/
│  ├─ apache/
│  ├─ php/
│  │  ├─ php-7.4.33-nts-x64/
│  │  ├─ php-8.1.32-nts-x64/
│  │  └─ php-8.3.20-nts-x64/
│  ├─ mysql/
│  ├─ redis/
│  └─ composer/
│
├─ data/
│  ├─ database/
│  │  └─ winserver.db
│  ├─ configs/
│  │  ├─ nginx/
│  │  ├─ apache/
│  │  ├─ php/
│  │  └─ mysql/
│  ├─ certificates/
│  ├─ backups/
│  └─ logs/
│
├─ templates/
│  ├─ nginx/
│  ├─ apache/
│  ├─ php/
│  └─ mysql/
│
├─ downloads/
├─ staging/
└─ temp/

最重要的原则：

程序文件、运行时、用户数据必须分离

应用更新不能覆盖：

data/

运行时升级不能覆盖旧运行时。

十五、SQLite 数据库设计

SQLite 足以承担本地管理器的数据存储。

建议核心表：

sites
site_domains
runtimes
runtime_instances
service_instances
port_bindings
certificates
operation_logs
runtime_packages
update_history
settings

例如服务实例：

CREATE TABLE service_instances (
    id TEXT PRIMARY KEY,
    runtime_id TEXT NOT NULL,
    instance_name TEXT NOT NULL,
    desired_state TEXT NOT NULL,
    actual_state TEXT NOT NULL,
    pid INTEGER,
    started_at TEXT,
    stopped_at TEXT,
    last_error_code TEXT,
    last_error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

SQLite 建议设置：

PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA busy_timeout = 5000;
PRAGMA synchronous = NORMAL;

WAL 模式允许读取和写入更好地并行，不过同一时间仍然只有一个 writer，因此建议只有 Agent 直接访问数据库，UI 全部通过 IPC 读取。SQLite 官方在 2026 年披露并修复了一个特定多连接并发 WAL 重置问题，因此如果使用 WAL，应固定到已修复版本，例如 SQLite 3.51.3 或相应回移版本之后，而不是依赖系统中未知版本的 SQLite。

十六、日志体系

日志需要分为三种。

系统日志
Agent 启动
IPC 连接
数据库迁移
权限问题
更新问题
服务日志
Nginx error.log
Nginx access.log
Apache error.log
PHP error.log
MySQL error.log
Redis log
操作日志
谁在什么时间
执行了什么操作
操作前状态
操作后状态
是否成功
失败原因

操作日志示例：

{
  "operationId": "op_20260611_001",
  "action": "site.switchPhp",
  "targetId": "site_123",
  "before": {
    "phpRuntimeId": "php-8.1.32"
  },
  "after": {
    "phpRuntimeId": "php-8.3.20"
  },
  "success": false,
  "errorCode": "NGINX_CONFIG_INVALID"
}
十七、诊断中心实现方案

升级版产品最大的竞争力不应该只是界面更漂亮，而应该是：

服务失败时，能够告诉用户为什么失败，以及怎么处理。

定义统一错误码：

PORT_OCCUPIED
CONFIG_INVALID
BINARY_NOT_FOUND
DEPENDENCY_MISSING
PROCESS_EXITED
HEALTH_CHECK_FAILED
HOSTS_PERMISSION_DENIED
RUNTIME_CORRUPTED
CERTIFICATE_EXPIRED
DATABASE_START_FAILED
PHP_EXTENSION_LOAD_FAILED

返回结构：

{
  "code": "PORT_OCCUPIED",
  "title": "Nginx 无法启动",
  "message": "80 端口已被其他程序占用",
  "details": {
    "port": 80,
    "pid": 5328,
    "processName": "iisexpress.exe"
  },
  "solutions": [
    {
      "type": "action",
      "label": "修改 Nginx 端口",
      "action": "nginx.changePort"
    },
    {
      "type": "guide",
      "label": "查看占用程序"
    }
  ]
}

以后可以在此基础上增加 AI，但第一阶段必须先用确定性规则完成诊断。

十八、推荐的项目代码结构
winserver/
├─ apps/
│  └─ desktop/
│     ├─ src/
│     │  ├─ pages/
│     │  ├─ components/
│     │  ├─ stores/
│     │  ├─ composables/
│     │  ├─ services/
│     │  └─ types/
│     └─ src-tauri/
│
├─ crates/
│  ├─ domain/
│  │  ├─ site/
│  │  ├─ runtime/
│  │  ├─ service/
│  │  └─ diagnostics/
│  │
│  ├─ application/
│  │  ├─ commands/
│  │  ├─ queries/
│  │  └─ workflows/
│  │
│  ├─ infrastructure/
│  │  ├─ database/
│  │  ├─ process/
│  │  ├─ windows/
│  │  ├─ filesystem/
│  │  ├─ network/
│  │  └─ templates/
│  │
│  ├─ adapters/
│  │  ├─ nginx/
│  │  ├─ apache/
│  │  ├─ php/
│  │  ├─ mysql/
│  │  └─ redis/
│  │
│  └─ ipc/
│
├─ services/
│  └─ agent/
│
├─ templates/
├─ runtime-manifests/
├─ migrations/
└─ tests/

这样 Desktop 和 Agent 可以共享：

domain
application
ipc
adapters

不会出现桌面端一套逻辑、后台服务又一套逻辑。

十九、推荐的开发阶段
第一阶段：最小闭环

只支持：

Nginx
PHP 7.4 / 8.1 / 8.3
网站创建
网站删除
PHP 切换
hosts 管理
启动停止
日志查看
端口检测

这一阶段验证完整链路：

创建网站
→ 启动 PHP
→ 生成 Nginx 配置
→ 启动 Nginx
→ 浏览器访问
→ 切换 PHP
→ 平滑重载
第二阶段：完善网站能力

增加：

Apache
HTTPS
本地 CA
伪静态模板
PHP 扩展管理
php.ini 编辑
Composer
终端
第三阶段：数据库体系

增加：

MySQL 多版本
MariaDB
Redis
数据库初始化
修改密码
备份恢复
数据目录迁移
第四阶段：组件市场与更新

增加：

在线下载
镜像源
组件签名
断点续传
版本回滚
运行时导入导出
环境快照
第五阶段：高级诊断

增加：

自动分析日志
依赖缺失检测
Visual C++ Runtime 检测
端口冲突处理
PHP 扩展冲突检测
配置修复建议
AI 辅助诊断