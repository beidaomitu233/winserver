using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using System.Runtime.Serialization;
using System.Text;
using System.Threading;
using System.Windows.Forms;

namespace XpCnLocalManager
{
#if WINFORMS_UI
    static class Program
    {
        [STAThread]
        static void Main()
        {
            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);
            Application.Run(new MainForm());
        }
    }
#endif

    [DataContract]
    public class AppConfig
    {
        [DataMember] public string PhpStudyRoot;
        [DataMember] public string WwwRoot;
        [DataMember] public string RedisRoot;
        [DataMember] public string MinioRoot;
        [DataMember] public string MinioDataDir;
        [DataMember] public string MysqlRootPassword;
        [DataMember] public List<SiteInfo> Sites;
        [DataMember] public List<DbInfo> Databases;
        [DataMember] public List<FtpInfo> FtpAccounts;
        [DataMember] public List<string> Logs;
    }

    [DataContract]
    public class SiteInfo
    {
        [DataMember] public string Domain;
        [DataMember] public string Port;
        [DataMember] public string Path;
        [DataMember] public string Status;
        [DataMember] public string Expire;
    }

    [DataContract]
    public class DbInfo
    {
        [DataMember] public string Db;
        [DataMember] public string User;
        [DataMember] public string Pass;
        [DataMember] public string Status;
    }

    [DataContract]
    public class FtpInfo
    {
        [DataMember] public string User;
        [DataMember] public string Path;
        [DataMember] public string Permission;
        [DataMember] public string Status;
    }

    public class ServiceInfo
    {
        public string Id;
        public string Name;
        public string Exe;
        public string Cwd;
        public string Args;
        public string ProcessName;
        public string ConfigFile;
        public int Port;
        public bool Auto;
        public Dictionary<string, string> Env = new Dictionary<string, string>();
    }

    public class SoftwareInfo
    {
        public string Id;
        public string Name;
        public string Category;
        public string Description;
        public string InstallDir;
        public string Executable;
        public string DownloadUrl;
        public bool RawDownload;
        public string RawFileName;
        public bool StripArchiveRoot;
        public string PostInstall;
        public string ServiceId;
        public string InstallNote;
    }

    public class ConfigFileInfo
    {
        public string Id;
        public string Label;
        public string Path;
    }

    public class ManagerCore
    {
        public readonly string RootDir;
        public readonly string DataDir;
        public readonly string RuntimeDir;
        public readonly string ConfigPath;
        public AppConfig Config;
        public List<ServiceInfo> Services;
        public List<SoftwareInfo> Software;
        public List<ConfigFileInfo> ConfigFiles;

        public ManagerCore(string rootDir)
        {
            RootDir = rootDir;
            DataDir = System.IO.Path.Combine(rootDir, "data");
            RuntimeDir = System.IO.Path.Combine(rootDir, "runtime");
            ConfigPath = System.IO.Path.Combine(DataDir, "config-winforms.xml");
            EnsureDirectory(DataDir);
            Load();
            BuildCatalogs();
        }

        public static string Slash(string value)
        {
            return (value ?? "").Replace('\\', '/');
        }

        public static void EnsureDirectory(string path)
        {
            if (!Directory.Exists(path)) Directory.CreateDirectory(path);
        }

        public static bool Exists(string path)
        {
            return !String.IsNullOrWhiteSpace(path) && (File.Exists(path) || Directory.Exists(path));
        }

        string DefaultPhpStudyRoot()
        {
            if (Directory.Exists(@"D:\phpstudy_pro")) return @"D:\phpstudy_pro";
            return System.IO.Path.Combine(RuntimeDir, "phpstudy_pro");
        }

        string DefaultRedisRoot()
        {
            if (File.Exists(@"D:\redis-windows-7.2.4\redis-server.exe")) return @"D:\redis-windows-7.2.4";
            return System.IO.Path.Combine(RuntimeDir, "components", "redis");
        }

        string DefaultMinioRoot()
        {
            if (File.Exists(@"D:\minio\minio.exe")) return @"D:\minio";
            return System.IO.Path.Combine(RuntimeDir, "components", "minio");
        }

        AppConfig NewDefaultConfig()
        {
            string phpstudy = DefaultPhpStudyRoot();
            string www = System.IO.Path.Combine(phpstudy, "WWW");
            string minioRoot = DefaultMinioRoot();
            string minioData = Directory.Exists(@"D:\minio") ? @"D:\minio" : System.IO.Path.Combine(minioRoot, "data");
            AppConfig cfg = new AppConfig();
            cfg.PhpStudyRoot = phpstudy;
            cfg.WwwRoot = www;
            cfg.RedisRoot = DefaultRedisRoot();
            cfg.MinioRoot = minioRoot;
            cfg.MinioDataDir = minioData;
            cfg.MysqlRootPassword = "";
            cfg.Sites = new List<SiteInfo>();
            cfg.Databases = new List<DbInfo>();
            cfg.FtpAccounts = new List<FtpInfo>();
            cfg.Logs = new List<string>();
            cfg.Sites.Add(new SiteInfo { Domain = "localhost", Port = "80", Path = Slash(System.IO.Path.Combine(www, "dist")), Status = "正常", Expire = "2035-12-03" });
            cfg.Databases.Add(new DbInfo { Db = "root", User = "root", Pass = "******", Status = "正常" });
            cfg.FtpAccounts.Add(new FtpInfo { User = "localhost", Path = Slash(www), Permission = "读写", Status = "正常" });
            return cfg;
        }

        void Load()
        {
            if (!File.Exists(ConfigPath))
            {
                Config = NewDefaultConfig();
                Save();
                return;
            }

            try
            {
                DataContractSerializer serializer = new DataContractSerializer(typeof(AppConfig));
                using (FileStream stream = File.OpenRead(ConfigPath))
                {
                    Config = (AppConfig)serializer.ReadObject(stream);
                }
            }
            catch
            {
                Config = NewDefaultConfig();
            }

            if (Config.Sites == null) Config.Sites = new List<SiteInfo>();
            if (Config.Databases == null) Config.Databases = new List<DbInfo>();
            if (Config.FtpAccounts == null) Config.FtpAccounts = new List<FtpInfo>();
            if (Config.Logs == null) Config.Logs = new List<string>();
            if (String.IsNullOrWhiteSpace(Config.PhpStudyRoot)) Config.PhpStudyRoot = DefaultPhpStudyRoot();
            if (String.IsNullOrWhiteSpace(Config.WwwRoot)) Config.WwwRoot = System.IO.Path.Combine(Config.PhpStudyRoot, "WWW");
            if (String.IsNullOrWhiteSpace(Config.RedisRoot)) Config.RedisRoot = DefaultRedisRoot();
            if (String.IsNullOrWhiteSpace(Config.MinioRoot)) Config.MinioRoot = DefaultMinioRoot();
            if (String.IsNullOrWhiteSpace(Config.MinioDataDir)) Config.MinioDataDir = Directory.Exists(@"D:\minio") ? @"D:\minio" : System.IO.Path.Combine(Config.MinioRoot, "data");
        }

        public void Save()
        {
            EnsureDirectory(DataDir);
            DataContractSerializer serializer = new DataContractSerializer(typeof(AppConfig));
            using (FileStream stream = File.Create(ConfigPath))
            {
                serializer.WriteObject(stream, Config);
            }
        }

        public void AddLog(string message)
        {
            Config.Logs.Insert(0, DateTime.Now.ToString("yyyy-MM-dd HH:mm:ss ") + message);
            while (Config.Logs.Count > 300) Config.Logs.RemoveAt(Config.Logs.Count - 1);
            Save();
        }

        string ExtRoot()
        {
            return System.IO.Path.Combine(Config.PhpStudyRoot, "Extensions");
        }

        string ApacheRoot()
        {
            return System.IO.Path.Combine(ExtRoot(), "Apache2.4.39");
        }

        string NginxRoot()
        {
            return System.IO.Path.Combine(ExtRoot(), "Nginx1.15.11");
        }

        string FtpRoot()
        {
            return System.IO.Path.Combine(ExtRoot(), "FTP0.9.60");
        }

        string Mysql57Root()
        {
            return System.IO.Path.Combine(ExtRoot(), "MySQL5.7.26");
        }

        string Mysql80Root()
        {
            return System.IO.Path.Combine(ExtRoot(), "MySQL8.0.12");
        }

        string PhpRoot()
        {
            return System.IO.Path.Combine(ExtRoot(), "php", "php7.3.4nts");
        }

        void BuildCatalogs()
        {
            Services = new List<ServiceInfo>();
            Software = new List<SoftwareInfo>();
            ConfigFiles = new List<ConfigFileInfo>();

            string apache = ApacheRoot();
            string nginx = NginxRoot();
            string ftp = FtpRoot();
            string mysql57 = Mysql57Root();
            string mysql80 = Mysql80Root();
            string php = PhpRoot();

            Services.Add(new ServiceInfo { Id = "apache", Name = "Apache2.4.39", Exe = System.IO.Path.Combine(apache, "bin", "httpd.exe"), Cwd = apache, Args = "-f \"" + System.IO.Path.Combine(apache, "conf", "httpd.conf") + "\"", ProcessName = "httpd", ConfigFile = System.IO.Path.Combine(apache, "conf", "httpd.conf"), Port = 80, Auto = true });
            Services.Add(new ServiceInfo { Id = "ftp", Name = "FTP0.9.60", Exe = System.IO.Path.Combine(ftp, "FileZilla Server.exe"), Cwd = ftp, Args = "", ProcessName = "FileZilla Server", ConfigFile = System.IO.Path.Combine(ftp, "FileZilla Server.xml"), Port = 21, Auto = false });
            Services.Add(new ServiceInfo { Id = "mysql57", Name = "MySQL5.7.26", Exe = System.IO.Path.Combine(mysql57, "bin", "mysqld.exe"), Cwd = mysql57, Args = "--defaults-file=\"" + System.IO.Path.Combine(mysql57, "my.ini") + "\"", ProcessName = "mysqld", ConfigFile = System.IO.Path.Combine(mysql57, "my.ini"), Port = ReadIniPort(System.IO.Path.Combine(mysql57, "my.ini"), 3307), Auto = false });
            Services.Add(new ServiceInfo { Id = "mysql80", Name = "MySQL8.0.12", Exe = System.IO.Path.Combine(mysql80, "bin", "mysqld.exe"), Cwd = mysql80, Args = "--defaults-file=\"" + System.IO.Path.Combine(mysql80, "my.ini") + "\"", ProcessName = "mysqld", ConfigFile = System.IO.Path.Combine(mysql80, "my.ini"), Port = ReadIniPort(System.IO.Path.Combine(mysql80, "my.ini"), 3306), Auto = true });
            Services.Add(new ServiceInfo { Id = "nginx", Name = "Nginx1.15.11", Exe = System.IO.Path.Combine(nginx, "nginx.exe"), Cwd = nginx, Args = "-p \"" + nginx + "\" -c conf/nginx.conf", ProcessName = "nginx", ConfigFile = System.IO.Path.Combine(nginx, "conf", "nginx.conf"), Port = 80, Auto = false });
            Services.Add(new ServiceInfo { Id = "redis", Name = "Redis7.2.4", Exe = System.IO.Path.Combine(Config.RedisRoot, "redis-server.exe"), Cwd = Config.RedisRoot, Args = "redis.conf", ProcessName = "redis-server", ConfigFile = System.IO.Path.Combine(Config.RedisRoot, "redis.conf"), Port = 6379, Auto = false });
            ServiceInfo minio = new ServiceInfo { Id = "minio", Name = "MinIO", Exe = System.IO.Path.Combine(Config.MinioRoot, "minio.exe"), Cwd = Config.MinioRoot, Args = "server \"" + Config.MinioDataDir + "\" --console-address \":9001\"", ProcessName = "minio", ConfigFile = System.IO.Path.Combine(Config.MinioRoot, "minio.env"), Port = 9000, Auto = false };
            minio.Env["MINIO_ROOT_USER"] = "minioadmin";
            minio.Env["MINIO_ROOT_PASSWORD"] = "minioadmin";
            Services.Add(minio);

            AddSoftware("apache", "Apache2.4.39", "Web Servers", "web服务，发布包内置", apache, System.IO.Path.Combine(apache, "bin", "httpd.exe"), "", false, "", false, "", "apache", "Apache Windows 二进制需要配置 Apache Lounge 或内网镜像 URL。");
            AddSoftware("nginx", "Nginx1.26.3", "Web Servers", "官方 Windows nginx 服务", nginx, System.IO.Path.Combine(nginx, "nginx.exe"), "https://nginx.org/download/nginx-1.26.3.zip", false, "", true, "", "nginx", "");
            AddSoftware("mysql80", "MySQL8.0", "数据库", "数据库服务", mysql80, System.IO.Path.Combine(mysql80, "bin", "mysqld.exe"), "https://dev.mysql.com/get/Downloads/MySQL-8.0/mysql-8.0.12-winx64.zip", false, "", true, "", "mysql80", "");
            AddSoftware("mysql57", "MySQL5.7", "数据库", "数据库服务", mysql57, System.IO.Path.Combine(mysql57, "bin", "mysqld.exe"), "https://dev.mysql.com/get/Downloads/MySQL-5.7/mysql-5.7.26-winx64.zip", false, "", true, "", "mysql57", "");
            AddSoftware("redis", "Redis7.2.4", "redis", "缓存、Session、队列与分布式锁服务", Config.RedisRoot, System.IO.Path.Combine(Config.RedisRoot, "redis-server.exe"), "https://www.nuget.org/api/v2/package/redis.windows.redist.x64/7.2.4", false, "", false, "redis-nuget", "redis", "");
            AddSoftware("minio", "MinIO", "对象存储", "兼容 S3 API 的本地对象存储服务", Config.MinioRoot, System.IO.Path.Combine(Config.MinioRoot, "minio.exe"), "https://dl.min.io/server/minio/release/windows-amd64/minio.exe", true, "minio.exe", false, "", "minio", "");
            AddSoftware("mc", "MinIO Client", "对象存储", "MinIO 命令行客户端", Config.MinioRoot, System.IO.Path.Combine(Config.MinioRoot, "mc.exe"), "https://dl.min.io/client/mc/release/windows-amd64/mc.exe", true, "mc.exe", false, "", "", "");
            AddSoftware("php73", "php7.3.33nts", "php", "PHP NTS 运行环境", php, System.IO.Path.Combine(php, "php-cgi.exe"), "https://windows.php.net/downloads/releases/archives/php-7.3.33-nts-Win32-VC15-x64.zip", false, "", false, "", "", "");
            AddSoftware("ftp", "FileZilla Server", "文件服务", "FTP 文件服务", ftp, System.IO.Path.Combine(ftp, "FileZilla Server.exe"), "", false, "", false, "", "ftp", "可配置 FileZilla Server 离线包或镜像 URL。");

            ConfigFiles.Add(new ConfigFileInfo { Id = "php.ini", Label = "php.ini", Path = System.IO.Path.Combine(php, "php.ini") });
            ConfigFiles.Add(new ConfigFileInfo { Id = "httpd.conf", Label = "httpd.conf", Path = System.IO.Path.Combine(apache, "conf", "httpd.conf") });
            ConfigFiles.Add(new ConfigFileInfo { Id = "nginx.conf", Label = "nginx.conf", Path = System.IO.Path.Combine(nginx, "conf", "nginx.conf") });
            ConfigFiles.Add(new ConfigFileInfo { Id = "vhosts.conf", Label = "vhosts.conf", Path = System.IO.Path.Combine(apache, "conf", "vhosts", "0localhost_80.conf") });
            ConfigFiles.Add(new ConfigFileInfo { Id = "mysql.ini", Label = "mysql.ini", Path = System.IO.Path.Combine(mysql80, "my.ini") });
            ConfigFiles.Add(new ConfigFileInfo { Id = "redis.conf", Label = "redis.conf", Path = System.IO.Path.Combine(Config.RedisRoot, "redis.conf") });
            ConfigFiles.Add(new ConfigFileInfo { Id = "minio.env", Label = "minio.env", Path = System.IO.Path.Combine(Config.MinioRoot, "minio.env") });
            ConfigFiles.Add(new ConfigFileInfo { Id = "hosts", Label = "hosts", Path = @"C:\Windows\System32\drivers\etc\hosts" });
        }

        void AddSoftware(string id, string name, string category, string desc, string dir, string exe, string url, bool raw, string rawName, bool stripRoot, string postInstall, string serviceId, string note)
        {
            Software.Add(new SoftwareInfo { Id = id, Name = name, Category = category, Description = desc, InstallDir = dir, Executable = exe, DownloadUrl = url, RawDownload = raw, RawFileName = rawName, StripArchiveRoot = stripRoot, PostInstall = postInstall, ServiceId = serviceId, InstallNote = note });
        }

        int ReadIniPort(string file, int fallback)
        {
            try
            {
                if (!File.Exists(file)) return fallback;
                foreach (string line in File.ReadAllLines(file))
                {
                    string trimmed = line.Trim();
                    if (trimmed.StartsWith("port=", StringComparison.OrdinalIgnoreCase))
                    {
                        int port;
                        if (Int32.TryParse(trimmed.Substring(5).Trim(), out port)) return port;
                    }
                }
            }
            catch { }
            return fallback;
        }

        public bool IsInstalled(SoftwareInfo item)
        {
            return File.Exists(item.Executable);
        }

        public bool IsRunning(ServiceInfo service)
        {
            string proc = service.ProcessName;
            if (proc.EndsWith(".exe", StringComparison.OrdinalIgnoreCase)) proc = proc.Substring(0, proc.Length - 4);
            bool inaccessibleMatch = false;
            try
            {
                Process[] processes = Process.GetProcessesByName(proc);
                foreach (Process p in processes)
                {
                    try
                    {
                        if (String.Equals(System.IO.Path.GetFullPath(p.MainModule.FileName), System.IO.Path.GetFullPath(service.Exe), StringComparison.OrdinalIgnoreCase)) return true;
                    }
                    catch
                    {
                        inaccessibleMatch = true;
                    }
                }
            }
            catch { }
            if (service.Id == "mysql57" || service.Id == "mysql80") return false;
            if (inaccessibleMatch) return true;
            return false;
        }

        bool IsPortOpen(int port)
        {
            try
            {
                using (TcpClient client = new TcpClient())
                {
                    IAsyncResult result = client.BeginConnect("127.0.0.1", port, null, null);
                    bool ok = result.AsyncWaitHandle.WaitOne(TimeSpan.FromMilliseconds(250));
                    if (!ok) return false;
                    client.EndConnect(result);
                    return true;
                }
            }
            catch
            {
                return false;
            }
        }

        public void StartService(ServiceInfo service)
        {
            if (!File.Exists(service.Exe)) throw new InvalidOperationException(service.Name + " 未安装：" + service.Exe);
            if (IsRunning(service))
            {
                AddLog(service.Name + " 已经在运行");
                return;
            }
            EnsureDirectory(service.Cwd);
            ProcessStartInfo psi = new ProcessStartInfo();
            psi.FileName = service.Exe;
            psi.Arguments = service.Args ?? "";
            psi.WorkingDirectory = service.Cwd;
            psi.UseShellExecute = false;
            psi.CreateNoWindow = true;
            psi.WindowStyle = ProcessWindowStyle.Hidden;
            foreach (KeyValuePair<string, string> item in service.Env) psi.EnvironmentVariables[item.Key] = item.Value;
            Process.Start(psi);
            Thread.Sleep(800);
            AddLog(service.Name + " 已启动");
        }

        public void StopService(ServiceInfo service)
        {
            if (service.Id == "nginx" && File.Exists(service.Exe))
            {
                try
                {
                    ProcessStartInfo stop = new ProcessStartInfo(service.Exe, "-p \"" + service.Cwd + "\" -s quit");
                    stop.WorkingDirectory = service.Cwd;
                    stop.UseShellExecute = false;
                    stop.CreateNoWindow = true;
                    Process.Start(stop).WaitForExit(1200);
                }
                catch { }
            }

            string proc = service.ProcessName;
            if (proc.EndsWith(".exe", StringComparison.OrdinalIgnoreCase)) proc = proc.Substring(0, proc.Length - 4);
            try
            {
                foreach (Process p in Process.GetProcessesByName(proc))
                {
                    bool same = true;
                    try
                    {
                        same = String.Equals(System.IO.Path.GetFullPath(p.MainModule.FileName), System.IO.Path.GetFullPath(service.Exe), StringComparison.OrdinalIgnoreCase);
                    }
                    catch { }
                    if (same)
                    {
                        try { p.Kill(); p.WaitForExit(2000); } catch { }
                    }
                }
            }
            catch { }
            AddLog(service.Name + " 已停止");
        }

        public void RestartService(ServiceInfo service)
        {
            StopService(service);
            Thread.Sleep(500);
            StartService(service);
        }

        public void StartSuite()
        {
            foreach (ServiceInfo service in Services)
            {
                if (!service.Auto) continue;
                if (service.Id == "apache" && IsPortOpen(80) && !IsRunning(service))
                {
                    AddLog("80端口已占用，跳过 Apache");
                    continue;
                }
                StartService(service);
            }
        }

        public void StopAllServices()
        {
            for (int i = Services.Count - 1; i >= 0; i--) StopService(Services[i]);
        }

        public void CreateSite(SiteInfo site)
        {
            if (String.IsNullOrWhiteSpace(site.Domain)) throw new InvalidOperationException("域名不能为空");
            if (String.IsNullOrWhiteSpace(site.Port)) site.Port = "80";
            if (String.IsNullOrWhiteSpace(site.Path)) site.Path = System.IO.Path.Combine(Config.WwwRoot, SafeName(site.Domain));
            site.Path = Slash(site.Path);
            site.Status = "正常";
            if (String.IsNullOrWhiteSpace(site.Expire)) site.Expire = "2035-12-03";
            EnsureDirectory(site.Path);
            EnsureListen(site.Port);

            string fileBase = Config.Sites.Count.ToString() + SafeName(site.Domain) + "_" + site.Port + ".conf";
            string apacheDir = System.IO.Path.Combine(ApacheRoot(), "conf", "vhosts");
            string nginxDir = System.IO.Path.Combine(NginxRoot(), "conf", "vhosts");
            if (Directory.Exists(apacheDir)) File.WriteAllText(System.IO.Path.Combine(apacheDir, fileBase), ApacheVhost(site), Encoding.UTF8);
            if (Directory.Exists(nginxDir)) File.WriteAllText(System.IO.Path.Combine(nginxDir, fileBase), NginxVhost(site), Encoding.UTF8);
            Config.Sites.Add(site);
            AddLog("网站 " + site.Domain + ":" + site.Port + " 已创建");
            Save();
        }

        void EnsureListen(string port)
        {
            string file = System.IO.Path.Combine(ApacheRoot(), "conf", "vhosts", "Listen.conf");
            if (!File.Exists(file)) return;
            string text = File.ReadAllText(file, Encoding.UTF8);
            if (text.IndexOf("Listen " + port, StringComparison.OrdinalIgnoreCase) < 0)
            {
                Backup(file);
                File.AppendAllText(file, Environment.NewLine + "Listen " + port + Environment.NewLine, Encoding.UTF8);
            }
        }

        string ApacheVhost(SiteInfo site)
        {
            string php = Slash(PhpRoot());
            string doc = Slash(site.Path);
            return "<VirtualHost _default_:" + site.Port + ">\r\n" +
                   "    ServerName " + site.Domain + "\r\n" +
                   "    DocumentRoot \"" + doc + "\"\r\n" +
                   "    FcgidInitialEnv PHPRC \"" + php + "\"\r\n" +
                   "    AddHandler fcgid-script .php\r\n" +
                   "    FcgidWrapper \"" + php + "/php-cgi.exe\" .php\r\n" +
                   "  <Directory \"" + doc + "\">\r\n" +
                   "      Options FollowSymLinks ExecCGI\r\n" +
                   "      AllowOverride All\r\n" +
                   "      Order allow,deny\r\n" +
                   "      Allow from all\r\n" +
                   "      Require all granted\r\n" +
                   "      DirectoryIndex index.php index.html\r\n" +
                   "  </Directory>\r\n" +
                   "</VirtualHost>\r\n";
        }

        string NginxVhost(SiteInfo site)
        {
            string doc = Slash(site.Path);
            return "server {\r\n" +
                   "        listen        " + site.Port + ";\r\n" +
                   "        server_name  " + site.Domain + ";\r\n" +
                   "        root   \"" + doc + "\";\r\n" +
                   "        location / {\r\n" +
                   "            index index.php index.html;\r\n" +
                   "            include " + doc + "/nginx.htaccess;\r\n" +
                   "            autoindex off;\r\n" +
                   "        }\r\n" +
                   "        location ~ \\.php(.*)$ {\r\n" +
                   "            fastcgi_pass   127.0.0.1:9000;\r\n" +
                   "            fastcgi_index  index.php;\r\n" +
                   "            include        fastcgi_params;\r\n" +
                   "        }\r\n" +
                   "}\r\n";
        }

        string SafeName(string value)
        {
            StringBuilder builder = new StringBuilder();
            foreach (char c in (value ?? "").Trim())
            {
                if (Char.IsLetterOrDigit(c) || c == '.' || c == '_' || c == '-') builder.Append(c);
                else builder.Append('_');
            }
            return builder.Length == 0 ? "site" : builder.ToString();
        }

        public void CreateDatabase(string db, string user, string password, string rootPassword)
        {
            ServiceInfo mysql = FindService("mysql80");
            string client = System.IO.Path.Combine(Mysql80Root(), "bin", "mysql.exe");
            if (!File.Exists(client)) throw new InvalidOperationException("未找到 mysql.exe：" + client);
            if (!String.IsNullOrWhiteSpace(rootPassword)) Config.MysqlRootPassword = rootPassword;
            string sql = "CREATE DATABASE IF NOT EXISTS `" + EscapeSqlIdentifier(db) + "` DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci; " +
                         "CREATE USER IF NOT EXISTS '" + EscapeSql(user) + "'@'localhost' IDENTIFIED BY '" + EscapeSql(password) + "'; " +
                         "GRANT ALL PRIVILEGES ON `" + EscapeSqlIdentifier(db) + "`.* TO '" + EscapeSql(user) + "'@'localhost'; FLUSH PRIVILEGES;";
            RunMysql(client, mysql.Port, sql, Config.MysqlRootPassword);
            Config.Databases.Add(new DbInfo { Db = db, User = user, Pass = "******", Status = "正常" });
            AddLog("数据库 " + db + " 已创建");
            Save();
        }

        public void SyncDatabasesFromMysql()
        {
            List<string> names = TryReadDatabaseNamesByClient();
            if (names.Count == 0) names = TryReadDatabaseNamesByDataDir();
            if (names.Count == 0) return;

            HashSet<string> existing = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            foreach (DbInfo db in Config.Databases) existing.Add(db.Db);
            foreach (string name in names)
            {
                if (String.IsNullOrWhiteSpace(name)) continue;
                if (IsSystemDatabase(name)) continue;
                if (existing.Contains(name)) continue;
                Config.Databases.Add(new DbInfo { Db = name, User = GuessUserForDatabase(name), Pass = "******", Status = "正常" });
                existing.Add(name);
            }
            Save();
        }

        List<string> TryReadDatabaseNamesByClient()
        {
            List<string> names = new List<string>();
            try
            {
                ServiceInfo mysql = FindService("mysql80");
                string client = System.IO.Path.Combine(Mysql80Root(), "bin", "mysql.exe");
                if (mysql == null || !File.Exists(client) || !IsRunning(mysql)) return names;
                string stdout = RunMysqlCapture(client, mysql.Port, "SHOW DATABASES;", Config.MysqlRootPassword);
                foreach (string line in stdout.Split(new char[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries))
                {
                    string value = line.Trim();
                    if (value.Length > 0 && !value.Equals("Database", StringComparison.OrdinalIgnoreCase)) names.Add(value);
                }
            }
            catch { }
            return names;
        }

        List<string> TryReadDatabaseNamesByDataDir()
        {
            List<string> names = new List<string>();
            try
            {
                string dataDir = ReadIniValue(System.IO.Path.Combine(Mysql80Root(), "my.ini"), "datadir");
                if (String.IsNullOrWhiteSpace(dataDir)) dataDir = System.IO.Path.Combine(Mysql80Root(), "data");
                dataDir = dataDir.Trim().Trim('"').Replace('/', '\\');
                if (!Directory.Exists(dataDir)) return names;
                foreach (string dir in Directory.GetDirectories(dataDir))
                {
                    string name = System.IO.Path.GetFileName(dir);
                    if (!IsSystemDatabase(name)) names.Add(name);
                }
            }
            catch { }
            return names;
        }

        string ReadIniValue(string file, string key)
        {
            if (!File.Exists(file)) return "";
            foreach (string line in File.ReadAllLines(file))
            {
                string trimmed = line.Trim();
                if (trimmed.StartsWith(key + "=", StringComparison.OrdinalIgnoreCase))
                    return trimmed.Substring(key.Length + 1).Trim();
            }
            return "";
        }

        bool IsSystemDatabase(string name)
        {
            return name.Equals("mysql", StringComparison.OrdinalIgnoreCase)
                || name.Equals("information_schema", StringComparison.OrdinalIgnoreCase)
                || name.Equals("performance_schema", StringComparison.OrdinalIgnoreCase)
                || name.Equals("sys", StringComparison.OrdinalIgnoreCase)
                || name.Equals("#innodb_temp", StringComparison.OrdinalIgnoreCase);
        }

        string GuessUserForDatabase(string db)
        {
            if (db.Equals("xh_blog", StringComparison.OrdinalIgnoreCase)) return "xinghuiTec";
            if (db.Equals("housego", StringComparison.OrdinalIgnoreCase)) return "xinghuiTec1";
            if (db.Equals("ruoyi", StringComparison.OrdinalIgnoreCase)) return "ruoyi";
            if (db.Equals("xinghui_admin", StringComparison.OrdinalIgnoreCase)) return "xinghuitec";
            return db;
        }

        public void ChangeRootPassword(string newPassword)
        {
            ServiceInfo mysql = FindService("mysql80");
            string client = System.IO.Path.Combine(Mysql80Root(), "bin", "mysql.exe");
            if (!File.Exists(client)) throw new InvalidOperationException("未找到 mysql.exe：" + client);
            string sql = "ALTER USER 'root'@'localhost' IDENTIFIED BY '" + EscapeSql(newPassword) + "'; FLUSH PRIVILEGES;";
            RunMysql(client, mysql.Port, sql, Config.MysqlRootPassword);
            Config.MysqlRootPassword = newPassword;
            AddLog("root 密码已修改");
            Save();
        }

        void RunMysql(string client, int port, string sql, string rootPassword)
        {
            ProcessStartInfo psi = new ProcessStartInfo();
            psi.FileName = client;
            psi.Arguments = "-uroot " + (String.IsNullOrEmpty(rootPassword) ? "" : "-p" + QuoteArg(rootPassword) + " ") + "-h 127.0.0.1 -P " + port.ToString() + " -e " + QuoteArg(sql);
            psi.UseShellExecute = false;
            psi.CreateNoWindow = true;
            psi.RedirectStandardError = true;
            psi.RedirectStandardOutput = true;
            Process p = Process.Start(psi);
            string stdout = p.StandardOutput.ReadToEnd();
            string stderr = p.StandardError.ReadToEnd();
            p.WaitForExit();
            if (p.ExitCode != 0) throw new InvalidOperationException(stderr.Length > 0 ? stderr : stdout);
        }

        string RunMysqlCapture(string client, int port, string sql, string rootPassword)
        {
            ProcessStartInfo psi = new ProcessStartInfo();
            psi.FileName = client;
            psi.Arguments = "-uroot " + (String.IsNullOrEmpty(rootPassword) ? "" : "-p" + QuoteArg(rootPassword) + " ") + "-N -B -h 127.0.0.1 -P " + port.ToString() + " -e " + QuoteArg(sql);
            psi.UseShellExecute = false;
            psi.CreateNoWindow = true;
            psi.RedirectStandardError = true;
            psi.RedirectStandardOutput = true;
            Process p = Process.Start(psi);
            string stdout = p.StandardOutput.ReadToEnd();
            string stderr = p.StandardError.ReadToEnd();
            p.WaitForExit();
            if (p.ExitCode != 0) throw new InvalidOperationException(stderr.Length > 0 ? stderr : stdout);
            return stdout;
        }

        string QuoteArg(string value)
        {
            return "\"" + (value ?? "").Replace("\\", "\\\\").Replace("\"", "\\\"") + "\"";
        }

        string EscapeSql(string value)
        {
            return (value ?? "").Replace("\\", "\\\\").Replace("'", "\\'");
        }

        string EscapeSqlIdentifier(string value)
        {
            return (value ?? "").Replace("`", "``");
        }

        public ServiceInfo FindService(string id)
        {
            foreach (ServiceInfo service in Services) if (service.Id == id) return service;
            return null;
        }

        public void CreateFtp(FtpInfo account)
        {
            if (String.IsNullOrWhiteSpace(account.User)) throw new InvalidOperationException("FTP 用户名不能为空");
            if (String.IsNullOrWhiteSpace(account.Path)) account.Path = Config.WwwRoot;
            account.Path = Slash(account.Path);
            account.Status = "正常";
            if (String.IsNullOrWhiteSpace(account.Permission)) account.Permission = "读写";
            EnsureDirectory(account.Path);
            Config.FtpAccounts.Add(account);
            AddLog("FTP账号 " + account.User + " 已创建");
            Save();
        }

        public string ReadConfig(ConfigFileInfo file)
        {
            if (!File.Exists(file.Path)) return "";
            return File.ReadAllText(file.Path, Encoding.UTF8);
        }

        public void SaveConfig(ConfigFileInfo file, string content)
        {
            EnsureDirectory(System.IO.Path.GetDirectoryName(file.Path));
            Backup(file.Path);
            File.WriteAllText(file.Path, content ?? "", Encoding.UTF8);
            AddLog(file.Label + " 已保存");
        }

        string Backup(string file)
        {
            if (!File.Exists(file)) return "";
            string backup = file + "." + DateTime.Now.ToString("yyyyMMddHHmmss") + ".bak";
            File.Copy(file, backup, true);
            return backup;
        }

        public void InstallSoftware(SoftwareInfo item, Action<string> progress)
        {
            if (File.Exists(item.Executable))
            {
                progress(item.Name + " 已安装");
                return;
            }
            if (String.IsNullOrWhiteSpace(item.DownloadUrl)) throw new InvalidOperationException(item.InstallNote.Length > 0 ? item.InstallNote : item.Name + " 未配置下载地址");
            EnsureDirectory(item.InstallDir);
            string downloads = System.IO.Path.Combine(RuntimeDir, "downloads");
            EnsureDirectory(downloads);
            progress("下载 " + item.Name);
            if (item.RawDownload)
            {
                string target = System.IO.Path.Combine(item.InstallDir, String.IsNullOrWhiteSpace(item.RawFileName) ? System.IO.Path.GetFileName(item.Executable) : item.RawFileName);
                using (WebClient client = new WebClient()) client.DownloadFile(item.DownloadUrl, target);
            }
            else
            {
                string archive = System.IO.Path.Combine(downloads, item.Id + ".zip");
                string extract = System.IO.Path.Combine(downloads, item.Id + "-extract");
                if (Directory.Exists(extract)) Directory.Delete(extract, true);
                using (WebClient client = new WebClient()) client.DownloadFile(item.DownloadUrl, archive);
                progress("解压 " + item.Name);
                ZipFile.ExtractToDirectory(archive, extract);
                string source = extract;
                if (item.StripArchiveRoot)
                {
                    string[] dirs = Directory.GetDirectories(extract);
                    if (dirs.Length == 1) source = dirs[0];
                }
                CopyDirectory(source, item.InstallDir);
                if (item.PostInstall == "redis-nuget") NormalizeRedis(item.InstallDir);
            }
            BuildCatalogs();
            AddLog(item.Name + " 已安装");
            progress(item.Name + " 已安装");
        }

        void NormalizeRedis(string installDir)
        {
            string[] candidates = new string[] { System.IO.Path.Combine(installDir, "tools"), System.IO.Path.Combine(installDir, "content"), installDir };
            foreach (string candidate in candidates)
            {
                if (File.Exists(System.IO.Path.Combine(candidate, "redis-server.exe")))
                {
                    if (!String.Equals(candidate, installDir, StringComparison.OrdinalIgnoreCase)) CopyDirectory(candidate, installDir);
                    string conf = System.IO.Path.Combine(installDir, "redis.conf");
                    if (!File.Exists(conf)) File.WriteAllText(conf, "port 6379\r\nbind 127.0.0.1\r\nappendonly yes\r\n", Encoding.UTF8);
                    return;
                }
            }
        }

        void CopyDirectory(string source, string destination)
        {
            EnsureDirectory(destination);
            foreach (string dir in Directory.GetDirectories(source, "*", SearchOption.AllDirectories))
            {
                EnsureDirectory(dir.Replace(source, destination));
            }
            foreach (string file in Directory.GetFiles(source, "*", SearchOption.AllDirectories))
            {
                string target = file.Replace(source, destination);
                EnsureDirectory(System.IO.Path.GetDirectoryName(target));
                File.Copy(file, target, true);
            }
        }
    }

#if WINFORMS_UI
    public class MainForm : Form
    {
        ManagerCore core;
        Panel sidebar;
        Panel content;
        StatusStrip status;
        ToolStripStatusLabel serviceStatus;
        ToolStripStatusLabel versionLabel;
        NotifyIcon tray;
        Dictionary<string, Button> navButtons = new Dictionary<string, Button>();
        DataGridView grid;
        TextBox logBox;
        TextBox configEditor;
        ComboBox configSelect;
        bool forceExit;

        public MainForm()
        {
            core = new ManagerCore(AppDomain.CurrentDomain.BaseDirectory);
            Text = "XP.CN 小皮 - 本地开发环境管理";
            StartPosition = FormStartPosition.CenterScreen;
            MinimumSize = new Size(980, 720);
            Size = new Size(1000, 787);
            Font = new Font("Microsoft YaHei UI", 10F);
            BuildShell();
            BuildTray();
            ShowHome();
            RefreshStatus();
        }

        void BuildShell()
        {
            sidebar = new Panel();
            sidebar.Dock = DockStyle.Left;
            sidebar.Width = 250;
            sidebar.BackColor = Color.FromArgb(24, 152, 226);
            Controls.Add(sidebar);

            Label logo = new Label();
            logo.Text = "XP.\r\nCN";
            logo.ForeColor = Color.White;
            logo.Font = new Font("Arial", 42F, FontStyle.Regular);
            logo.SetBounds(28, 6, 180, 88);
            sidebar.Controls.Add(logo);

            AddNav("home", "首页", 102);
            AddNav("website", "网站", 177);
            AddNav("database", "数据库", 252);
            AddNav("ftp", "FTP", 327);
            AddNav("software", "软件管理", 402);
            AddNav("settings", "设置", 477);

            Panel header = new Panel();
            header.Dock = DockStyle.Top;
            header.Height = 62;
            header.BackColor = Color.White;
            Controls.Add(header);
            header.BringToFront();

            Label notice = new Label();
            notice.Text = "高性能云服务器最低15元/月  QQ群：176643616";
            notice.AutoSize = false;
            notice.TextAlign = ContentAlignment.MiddleLeft;
            notice.SetBounds(18, 0, 600, 62);
            header.Controls.Add(notice);

            content = new Panel();
            content.Dock = DockStyle.Fill;
            content.BackColor = Color.FromArgb(246, 246, 246);
            content.Padding = new Padding(10);
            Controls.Add(content);
            content.BringToFront();

            status = new StatusStrip();
            serviceStatus = new ToolStripStatusLabel();
            versionLabel = new ToolStripStatusLabel("版本：8.1.1.3-local");
            serviceStatus.Spring = true;
            serviceStatus.TextAlign = ContentAlignment.MiddleLeft;
            status.Items.Add(serviceStatus);
            status.Items.Add(versionLabel);
            Controls.Add(status);
            status.BringToFront();
        }

        void AddNav(string id, string text, int top)
        {
            Button button = new Button();
            button.Text = text;
            button.Tag = id;
            button.FlatStyle = FlatStyle.Flat;
            button.FlatAppearance.BorderSize = 0;
            button.TextAlign = ContentAlignment.MiddleLeft;
            button.Padding = new Padding(66, 0, 0, 0);
            button.Font = new Font("Microsoft YaHei UI", 20F, FontStyle.Bold);
            button.SetBounds(0, top, 250, 75);
            button.ForeColor = Color.White;
            button.BackColor = Color.FromArgb(24, 152, 226);
            button.Click += delegate { Navigate(id); };
            sidebar.Controls.Add(button);
            navButtons[id] = button;
        }

        void BuildTray()
        {
            ContextMenuStrip menu = new ContextMenuStrip();
            menu.Items.Add("打开主窗口", null, delegate { ShowFromTray(); });
            menu.Items.Add("一键启动", null, delegate { SafeRun(delegate { core.StartSuite(); ShowHome(); }); });
            menu.Items.Add("停止全部服务", null, delegate { SafeRun(delegate { core.StopAllServices(); RefreshStatus(); }); });
            menu.Items.Add(new ToolStripSeparator());
            menu.Items.Add("退出", null, delegate { ExitApp(); });

            tray = new NotifyIcon();
            tray.Icon = SystemIcons.Application;
            tray.Text = "XP.CN 小皮";
            tray.Visible = true;
            tray.ContextMenuStrip = menu;
            tray.DoubleClick += delegate { ShowFromTray(); };
        }

        void Navigate(string id)
        {
            if (id == "home") ShowHome();
            if (id == "website") ShowWebsite();
            if (id == "database") ShowDatabase();
            if (id == "ftp") ShowFtp();
            if (id == "software") ShowSoftware();
            if (id == "settings") ShowSettings();
        }

        void MarkNav(string id)
        {
            foreach (KeyValuePair<string, Button> item in navButtons)
            {
                item.Value.BackColor = item.Key == id ? Color.White : Color.FromArgb(24, 152, 226);
                item.Value.ForeColor = item.Key == id ? Color.FromArgb(24, 152, 226) : Color.White;
            }
        }

        void ClearContent(string nav)
        {
            MarkNav(nav);
            content.Controls.Clear();
            grid = null;
            logBox = null;
            configEditor = null;
            configSelect = null;
        }

        Button Primary(string text, int x, int y, int w, int h)
        {
            Button b = new Button();
            b.Text = text;
            b.SetBounds(x, y, w, h);
            b.BackColor = Color.FromArgb(24, 152, 226);
            b.ForeColor = Color.White;
            b.FlatStyle = FlatStyle.Flat;
            b.FlatAppearance.BorderSize = 0;
            return b;
        }

        void ShowHome()
        {
            ClearContent("home");
            Panel quick = PanelBox(0, 0, content.Width - 25, 110);
            quick.Anchor = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right;
            Label title = SectionTitle("一键启动", 28, 16);
            quick.Controls.Add(title);
            Button suite = Primary("WNMP 启动/停止", 30, 54, 170, 40);
            suite.Click += delegate { SafeRun(delegate { if (core.Services.Any(s => s.Auto && core.IsRunning(s))) core.StopAllServices(); else core.StartSuite(); ShowHome(); }); };
            quick.Controls.Add(suite);
            Button dbTool = Primary("打开数据库工具", 220, 54, 150, 40);
            dbTool.Click += delegate { Process.Start("http://127.0.0.1/phpmyadmin"); };
            quick.Controls.Add(dbTool);
            content.Controls.Add(quick);

            Panel servicesPanel = PanelBox(0, 120, content.Width - 25, 360);
            servicesPanel.Anchor = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right;
            servicesPanel.Controls.Add(SectionTitle("套件", 28, 16));
            DataGridView serviceGrid = Grid(30, 54, servicesPanel.Width - 60, 285);
            serviceGrid.Anchor = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right | AnchorStyles.Bottom;
            serviceGrid.Columns.Add("name", "服务");
            serviceGrid.Columns.Add("status", "状态");
            serviceGrid.Columns.Add(ButtonCol("toggle", "启动/停止"));
            serviceGrid.Columns.Add(ButtonCol("restart", "重启"));
            serviceGrid.Columns.Add(ButtonCol("config", "配置"));
            foreach (ServiceInfo s in core.Services)
            {
                serviceGrid.Rows.Add(s.Name, core.IsRunning(s) ? "运行中" : (File.Exists(s.Exe) ? "已停止" : "未安装"), core.IsRunning(s) ? "停止" : "启动", "重启", "配置");
            }
            serviceGrid.CellContentClick += delegate(object sender, DataGridViewCellEventArgs e)
            {
                if (e.RowIndex < 0 || e.ColumnIndex < 2) return;
                ServiceInfo s = core.Services[e.RowIndex];
                SafeRun(delegate
                {
                    if (e.ColumnIndex == 2)
                    {
                        if (core.IsRunning(s)) core.StopService(s); else core.StartService(s);
                    }
                    if (e.ColumnIndex == 3) core.RestartService(s);
                    if (e.ColumnIndex == 4) OpenConfigForPath(s.ConfigFile);
                    ShowHome();
                });
            };
            servicesPanel.Controls.Add(serviceGrid);
            content.Controls.Add(servicesPanel);

            Panel logs = PanelBox(0, 490, content.Width - 25, 150);
            logs.Anchor = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right | AnchorStyles.Bottom;
            logs.Controls.Add(SectionTitle("运行状态", 28, 12));
            logBox = new TextBox();
            logBox.Multiline = true;
            logBox.ReadOnly = true;
            logBox.ScrollBars = ScrollBars.Vertical;
            logBox.BorderStyle = BorderStyle.None;
            logBox.BackColor = Color.FromArgb(247, 247, 247);
            logBox.ForeColor = Color.FromArgb(0, 143, 255);
            logBox.SetBounds(30, 44, logs.Width - 60, logs.Height - 58);
            logBox.Anchor = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right | AnchorStyles.Bottom;
            logBox.Text = String.Join(Environment.NewLine, core.Config.Logs.ToArray());
            logs.Controls.Add(logBox);
            content.Controls.Add(logs);
            RefreshStatus();
        }

        Panel PanelBox(int x, int y, int w, int h)
        {
            Panel p = new Panel();
            p.SetBounds(x, y, w, h);
            p.BackColor = Color.White;
            return p;
        }

        Label SectionTitle(string text, int x, int y)
        {
            Label label = new Label();
            label.Text = text;
            label.Font = new Font("Microsoft YaHei UI", 12F);
            label.SetBounds(x, y, 220, 26);
            return label;
        }

        DataGridView Grid(int x, int y, int w, int h)
        {
            DataGridView g = new DataGridView();
            g.SetBounds(x, y, w, h);
            g.AllowUserToAddRows = false;
            g.AllowUserToDeleteRows = false;
            g.AutoSizeColumnsMode = DataGridViewAutoSizeColumnsMode.Fill;
            g.RowHeadersVisible = false;
            g.SelectionMode = DataGridViewSelectionMode.FullRowSelect;
            g.BackgroundColor = Color.White;
            g.BorderStyle = BorderStyle.None;
            return g;
        }

        DataGridViewButtonColumn ButtonCol(string name, string text)
        {
            DataGridViewButtonColumn col = new DataGridViewButtonColumn();
            col.Name = name;
            col.HeaderText = text;
            col.UseColumnTextForButtonValue = false;
            return col;
        }

        void ShowWebsite()
        {
            ClearContent("website");
            Button create = Primary("+ 创建网站", 0, 0, 110, 36);
            create.Click += delegate { CreateSiteDialog(); };
            content.Controls.Add(create);
            grid = Grid(0, 46, content.Width - 25, content.Height - 70);
            grid.Anchor = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right | AnchorStyles.Bottom;
            grid.Columns.Add("idx", "序号");
            grid.Columns.Add("domain", "网站域名");
            grid.Columns.Add("port", "端口");
            grid.Columns.Add("path", "物理路径");
            grid.Columns.Add("status", "状态");
            grid.Columns.Add("expire", "到期");
            grid.Columns.Add(ButtonCol("manage", "操作"));
            for (int i = 0; i < core.Config.Sites.Count; i++)
            {
                SiteInfo s = core.Config.Sites[i];
                grid.Rows.Add((i + 1).ToString(), s.Domain, s.Port, s.Path, s.Status, s.Expire, "管理");
            }
            grid.CellContentClick += delegate(object sender, DataGridViewCellEventArgs e)
            {
                if (e.RowIndex >= 0 && e.ColumnIndex == 6) OpenFolder(core.Config.Sites[e.RowIndex].Path);
            };
            content.Controls.Add(grid);
        }

        void ShowDatabase()
        {
            ClearContent("database");
            Button create = Primary("+ 创建数据库", 0, 0, 130, 36);
            create.Click += delegate { CreateDatabaseDialog(); };
            content.Controls.Add(create);
            Button root = Primary("修改root密码", 140, 0, 130, 36);
            root.Click += delegate { ChangeRootPasswordDialog(); };
            content.Controls.Add(root);
            grid = Grid(0, 46, content.Width - 25, content.Height - 70);
            grid.Anchor = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right | AnchorStyles.Bottom;
            grid.Columns.Add("idx", "序号");
            grid.Columns.Add("db", "数据库");
            grid.Columns.Add("user", "用户");
            grid.Columns.Add("pass", "密码");
            grid.Columns.Add("status", "状态");
            for (int i = 0; i < core.Config.Databases.Count; i++)
            {
                DbInfo db = core.Config.Databases[i];
                grid.Rows.Add((i + 1).ToString(), db.Db, db.User, db.Pass, db.Status);
            }
            content.Controls.Add(grid);
        }

        void ShowFtp()
        {
            ClearContent("ftp");
            Button create = Primary("+ 创建FTP", 0, 0, 110, 36);
            create.Click += delegate { CreateFtpDialog(); };
            content.Controls.Add(create);
            grid = Grid(0, 46, content.Width - 25, content.Height - 70);
            grid.Anchor = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right | AnchorStyles.Bottom;
            grid.Columns.Add("idx", "序号");
            grid.Columns.Add("user", "用户名");
            grid.Columns.Add("path", "根目录");
            grid.Columns.Add("permission", "权限");
            grid.Columns.Add("status", "状态");
            for (int i = 0; i < core.Config.FtpAccounts.Count; i++)
            {
                FtpInfo f = core.Config.FtpAccounts[i];
                grid.Rows.Add((i + 1).ToString(), f.User, f.Path, f.Permission, f.Status);
            }
            content.Controls.Add(grid);
        }

        void ShowSoftware()
        {
            ClearContent("software");
            grid = Grid(0, 0, content.Width - 25, content.Height - 30);
            grid.Anchor = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right | AnchorStyles.Bottom;
            grid.Columns.Add("name", "软件");
            grid.Columns.Add("category", "分类");
            grid.Columns.Add("desc", "说明");
            grid.Columns.Add("state", "状态");
            grid.Columns.Add(ButtonCol("install", "安装/卸载"));
            grid.Columns.Add(ButtonCol("settings", "设置"));
            foreach (SoftwareInfo item in core.Software)
            {
                bool installed = core.IsInstalled(item);
                grid.Rows.Add(item.Name, item.Category, item.Description, installed ? "已安装" : "未安装", installed ? "已安装" : "安装", "设置");
            }
            grid.CellContentClick += delegate(object sender, DataGridViewCellEventArgs e)
            {
                if (e.RowIndex < 0) return;
                SoftwareInfo item = core.Software[e.RowIndex];
                if (e.ColumnIndex == 4)
                {
                    if (core.IsInstalled(item))
                    {
                        MessageBox.Show(item.Name + " 已安装。为避免误删外部目录，暂不提供自动卸载。", "提示");
                        return;
                    }
                    SafeRun(delegate
                    {
                        UseWaitCursor = true;
                        core.InstallSoftware(item, delegate(string msg) { core.AddLog(msg); });
                        UseWaitCursor = false;
                        ShowSoftware();
                    });
                }
                if (e.ColumnIndex == 5)
                {
                    OpenFolder(item.InstallDir);
                }
            };
            content.Controls.Add(grid);
        }

        void ShowSettings()
        {
            ClearContent("settings");
            configSelect = new ComboBox();
            configSelect.DropDownStyle = ComboBoxStyle.DropDownList;
            configSelect.SetBounds(0, 0, 220, 32);
            foreach (ConfigFileInfo f in core.ConfigFiles) configSelect.Items.Add(f.Label);
            configSelect.SelectedIndexChanged += delegate { LoadSelectedConfig(); };
            content.Controls.Add(configSelect);
            Button save = Primary("保存配置", 230, 0, 110, 32);
            save.Click += delegate
            {
                SafeRun(delegate
                {
                    if (configSelect.SelectedIndex < 0) return;
                    core.SaveConfig(core.ConfigFiles[configSelect.SelectedIndex], configEditor.Text);
                    MessageBox.Show("已保存，相关服务可能需要重启后生效。", "完成");
                });
            };
            content.Controls.Add(save);

            configEditor = new TextBox();
            configEditor.Multiline = true;
            configEditor.ScrollBars = ScrollBars.Both;
            configEditor.AcceptsTab = true;
            configEditor.Font = new Font("Consolas", 10F);
            configEditor.SetBounds(0, 42, content.Width - 25, content.Height - 70);
            configEditor.Anchor = AnchorStyles.Top | AnchorStyles.Left | AnchorStyles.Right | AnchorStyles.Bottom;
            content.Controls.Add(configEditor);
            if (configSelect.Items.Count > 0) configSelect.SelectedIndex = 0;
        }

        void LoadSelectedConfig()
        {
            if (configSelect.SelectedIndex < 0) return;
            ConfigFileInfo file = core.ConfigFiles[configSelect.SelectedIndex];
            try
            {
                configEditor.Text = core.ReadConfig(file);
            }
            catch (Exception ex)
            {
                configEditor.Text = "";
                MessageBox.Show(ex.Message, "读取配置失败");
            }
        }

        void OpenConfigForPath(string path)
        {
            ShowSettings();
            for (int i = 0; i < core.ConfigFiles.Count; i++)
            {
                if (String.Equals(System.IO.Path.GetFullPath(core.ConfigFiles[i].Path), System.IO.Path.GetFullPath(path), StringComparison.OrdinalIgnoreCase))
                {
                    configSelect.SelectedIndex = i;
                    return;
                }
            }
            Process.Start("notepad.exe", path);
        }

        void CreateSiteDialog()
        {
            using (InputDialog d = new InputDialog("创建网站", new string[] { "域名", "端口", "根目录" }, new string[] { "demo.local", "80", ManagerCore.Slash(System.IO.Path.Combine(core.Config.WwwRoot, "demo")) }))
            {
                if (d.ShowDialog(this) != DialogResult.OK) return;
                SafeRun(delegate { core.CreateSite(new SiteInfo { Domain = d.Values[0], Port = d.Values[1], Path = d.Values[2] }); ShowWebsite(); });
            }
        }

        void CreateDatabaseDialog()
        {
            using (InputDialog d = new InputDialog("创建数据库", new string[] { "数据库", "用户", "密码", "root密码(可空)" }, new string[] { "demo_app", "demo_app", "123456", core.Config.MysqlRootPassword }, new bool[] { false, false, true, true }))
            {
                if (d.ShowDialog(this) != DialogResult.OK) return;
                SafeRun(delegate { core.CreateDatabase(d.Values[0], d.Values[1], d.Values[2], d.Values[3]); ShowDatabase(); });
            }
        }

        void ChangeRootPasswordDialog()
        {
            using (InputDialog d = new InputDialog("修改root密码", new string[] { "新密码" }, new string[] { "" }, new bool[] { true }))
            {
                if (d.ShowDialog(this) != DialogResult.OK) return;
                SafeRun(delegate { core.ChangeRootPassword(d.Values[0]); ShowDatabase(); });
            }
        }

        void CreateFtpDialog()
        {
            using (InputDialog d = new InputDialog("创建FTP", new string[] { "用户名", "根目录", "权限" }, new string[] { "demo_ftp", ManagerCore.Slash(core.Config.WwwRoot), "读写" }))
            {
                if (d.ShowDialog(this) != DialogResult.OK) return;
                SafeRun(delegate { core.CreateFtp(new FtpInfo { User = d.Values[0], Path = d.Values[1], Permission = d.Values[2] }); ShowFtp(); });
            }
        }

        void SafeRun(Action action)
        {
            try
            {
                action();
                RefreshStatus();
            }
            catch (Exception ex)
            {
                core.AddLog("错误：" + ex.Message);
                MessageBox.Show(ex.Message, "操作失败", MessageBoxButtons.OK, MessageBoxIcon.Error);
                UseWaitCursor = false;
            }
        }

        void RefreshStatus()
        {
            List<string> parts = new List<string>();
            foreach (ServiceInfo s in core.Services)
            {
                if (s.Id == "apache" || s.Id == "mysql80" || s.Id == "redis" || s.Id == "minio")
                {
                    parts.Add((core.IsRunning(s) ? "▶ " : "■ ") + s.Name);
                }
            }
            serviceStatus.Text = String.Join("    ", parts.ToArray());
        }

        void OpenFolder(string folder)
        {
            if (!Directory.Exists(folder)) Directory.CreateDirectory(folder);
            Process.Start("explorer.exe", folder);
        }

        void ShowFromTray()
        {
            Show();
            WindowState = FormWindowState.Normal;
            Activate();
        }

        protected override void OnResize(EventArgs e)
        {
            base.OnResize(e);
            if (WindowState == FormWindowState.Minimized)
            {
                Hide();
                tray.ShowBalloonTip(1200, "XP.CN 小皮", "应用已最小化到托盘，服务保持运行。", ToolTipIcon.Info);
            }
        }

        protected override void OnFormClosing(FormClosingEventArgs e)
        {
            if (forceExit)
            {
                tray.Visible = false;
                base.OnFormClosing(e);
                return;
            }

            e.Cancel = true;
            using (ExitDialog dialog = new ExitDialog())
            {
                DialogResult result = dialog.ShowDialog(this);
                if (result == DialogResult.Cancel) return;
                if (dialog.CloseServers)
                {
                    SafeRun(delegate { core.StopAllServices(); });
                }
                forceExit = true;
                tray.Visible = false;
                Application.Exit();
            }
        }

        void ExitApp()
        {
            Close();
        }
    }

    public class InputDialog : Form
    {
        public string[] Values;
        TextBox[] boxes;

        public InputDialog(string title, string[] labels, string[] defaults) : this(title, labels, defaults, null) { }

        public InputDialog(string title, string[] labels, string[] defaults, bool[] password)
        {
            Text = title;
            StartPosition = FormStartPosition.CenterParent;
            FormBorderStyle = FormBorderStyle.FixedDialog;
            MinimizeBox = false;
            MaximizeBox = false;
            ClientSize = new Size(460, 76 + labels.Length * 42);
            Font = new Font("Microsoft YaHei UI", 10F);
            boxes = new TextBox[labels.Length];
            Values = new string[labels.Length];

            for (int i = 0; i < labels.Length; i++)
            {
                Label label = new Label();
                label.Text = labels[i];
                label.SetBounds(18, 18 + i * 42, 120, 28);
                label.TextAlign = ContentAlignment.MiddleRight;
                Controls.Add(label);

                TextBox box = new TextBox();
                box.SetBounds(150, 18 + i * 42, 280, 28);
                box.Text = defaults != null && defaults.Length > i ? defaults[i] : "";
                if (password != null && password.Length > i && password[i]) box.PasswordChar = '*';
                Controls.Add(box);
                boxes[i] = box;
            }

            Button ok = new Button();
            ok.Text = "确定";
            ok.DialogResult = DialogResult.OK;
            ok.SetBounds(260, ClientSize.Height - 46, 80, 30);
            Controls.Add(ok);

            Button cancel = new Button();
            cancel.Text = "取消";
            cancel.DialogResult = DialogResult.Cancel;
            cancel.SetBounds(350, ClientSize.Height - 46, 80, 30);
            Controls.Add(cancel);

            AcceptButton = ok;
            CancelButton = cancel;
        }

        protected override void OnFormClosing(FormClosingEventArgs e)
        {
            base.OnFormClosing(e);
            for (int i = 0; i < boxes.Length; i++) Values[i] = boxes[i].Text;
        }
    }

    public class ExitDialog : Form
    {
        public bool CloseServers;

        public ExitDialog()
        {
            Text = "退出 XP.CN 小皮";
            StartPosition = FormStartPosition.CenterParent;
            FormBorderStyle = FormBorderStyle.FixedDialog;
            MinimizeBox = false;
            MaximizeBox = false;
            ClientSize = new Size(470, 170);
            Font = new Font("Microsoft YaHei UI", 10F);

            Label text = new Label();
            text.Text = "退出应用时是否关闭所有 server 服务？\r\n可以选择保留 MySQL、Redis、MinIO 等服务在后台继续运行。";
            text.SetBounds(22, 22, 425, 70);
            Controls.Add(text);

            Button closeAll = new Button();
            closeAll.Text = "关闭全部并退出";
            closeAll.SetBounds(32, 110, 130, 34);
            closeAll.Click += delegate { CloseServers = true; DialogResult = DialogResult.OK; Close(); };
            Controls.Add(closeAll);

            Button keep = new Button();
            keep.Text = "保留后台服务";
            keep.SetBounds(174, 110, 130, 34);
            keep.Click += delegate { CloseServers = false; DialogResult = DialogResult.OK; Close(); };
            Controls.Add(keep);

            Button cancel = new Button();
            cancel.Text = "取消";
            cancel.SetBounds(316, 110, 110, 34);
            cancel.Click += delegate { DialogResult = DialogResult.Cancel; Close(); };
            Controls.Add(cancel);
        }
    }
#endif
}
