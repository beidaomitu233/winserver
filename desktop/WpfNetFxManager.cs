using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using Controls = System.Windows.Controls;
using Data = System.Windows.Data;
using Input = System.Windows.Input;
using Media = System.Windows.Media;
using Forms = System.Windows.Forms;

namespace XpCnLocalManager
{
    public class WpfNetFxApp : Application
    {
        [STAThread]
        public static void Main()
        {
            WpfNetFxApp app = new WpfNetFxApp();
            app.Run(new WpfNetFxWindow());
        }
    }

    public class WpfNetFxWindow : Window
    {
        readonly ManagerCore core;
        readonly Dictionary<string, Controls.Button> navButtons = new Dictionary<string, Controls.Button>();
        readonly Controls.Grid content = new Controls.Grid();
        readonly Controls.StackPanel navPanel = new Controls.StackPanel();
        readonly Controls.TextBlock title = new Controls.TextBlock();
        readonly Controls.TextBlock runtimeText = new Controls.TextBlock();
        readonly Controls.TextBlock statusText = new Controls.TextBlock();
        Forms.NotifyIcon tray;
        bool dark = true;
        bool forceExit;
        string currentView = "home";

        Media.Brush Bg, Side, Surface, Card, Elevated, Border, Text, Muted, Ghost, Accent, Good, Bad;

        public WpfNetFxWindow()
        {
            core = new ManagerCore(AppRoot());
            Title = "Codex Server Control";
            Width = 1160;
            Height = 760;
            MinWidth = 980;
            MinHeight = 680;
            WindowStartupLocation = WindowStartupLocation.CenterScreen;
            FontFamily = new Media.FontFamily("Microsoft YaHei UI");
            BuildTray();
            ApplyTheme();
            BuildShell();
            Navigate("home");
        }

        static string AppRoot()
        {
            DirectoryInfo dir = new DirectoryInfo(AppDomain.CurrentDomain.BaseDirectory.TrimEnd('\\'));
            for (int i = 0; i < 6 && dir != null; i++, dir = dir.Parent)
            {
                if (Directory.Exists(Path.Combine(dir.FullName, "desktop"))) return dir.FullName;
            }
            return AppDomain.CurrentDomain.BaseDirectory;
        }

        Media.SolidColorBrush Brush(string value)
        {
            return (Media.SolidColorBrush)new Media.BrushConverter().ConvertFromString(value);
        }

        void ApplyTheme()
        {
            Bg = Brush(dark ? "#0F1117" : "#F5F7FA");
            Side = Brush(dark ? "#161B24" : "#FFFFFF");
            Surface = Brush(dark ? "#11151D" : "#FFFFFF");
            Card = Brush(dark ? "#171C26" : "#FFFFFF");
            Elevated = Brush(dark ? "#202838" : "#EEF3F8");
            Border = Brush(dark ? "#29313D" : "#DDE4EE");
            Text = Brush(dark ? "#F2F5F8" : "#111827");
            Muted = Brush(dark ? "#94A3B8" : "#6B7280");
            Ghost = Brush(dark ? "#252D3B" : "#E8EEF6");
            Accent = Brush("#188CE5");
            Good = Brush("#35C58A");
            Bad = Brush("#EF4444");
            Background = Bg;
        }

        void BuildShell()
        {
            Controls.Grid root = new Controls.Grid();
            root.ColumnDefinitions.Add(new Controls.ColumnDefinition { Width = new GridLength(248) });
            root.ColumnDefinitions.Add(new Controls.ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            Content = root;

            Controls.Border sidebar = new Controls.Border { Background = Side, BorderBrush = Border, BorderThickness = new Thickness(0, 0, 1, 0) };
            Controls.Grid.SetColumn(sidebar, 0);
            root.Children.Add(sidebar);

            Controls.DockPanel sideDock = new Controls.DockPanel();
            sidebar.Child = sideDock;

            Controls.StackPanel brand = new Controls.StackPanel { Margin = new Thickness(22) };
            Controls.DockPanel.SetDock(brand, Dock.Top);
            sideDock.Children.Add(brand);
            Controls.Border icon = new Controls.Border { Width = 42, Height = 42, CornerRadius = new CornerRadius(10), Background = Accent, HorizontalAlignment = HorizontalAlignment.Left };
            icon.Child = new Controls.TextBlock { Text = "P", Foreground = Media.Brushes.White, FontSize = 25, FontWeight = FontWeights.SemiBold, HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center };
            brand.Children.Add(icon);
            brand.Children.Add(Txt("Codex Server", 20, FontWeights.SemiBold, new Thickness(0, 14, 0, 0)));
            brand.Children.Add(Mute("Local web stack manager", 12, new Thickness(0, 4, 0, 0)));

            navPanel.Margin = new Thickness(0, 4, 0, 0);
            Controls.DockPanel.SetDock(navPanel, Dock.Top);
            sideDock.Children.Add(navPanel);
            AddNav("home", "⌂  首页");
            AddNav("website", "◎  网站");
            AddNav("database", "◉  数据库");
            AddNav("ftp", "▣  FTP");
            AddNav("software", "☷  软件管理");
            AddNav("settings", "⚙  设置");

            Controls.Border runtime = new Controls.Border { Margin = new Thickness(18), Padding = new Thickness(14), CornerRadius = new CornerRadius(10), Background = Card, BorderBrush = Border, BorderThickness = new Thickness(1) };
            Controls.DockPanel.SetDock(runtime, Dock.Bottom);
            sideDock.Children.Add(runtime);
            Controls.StackPanel runtimeStack = new Controls.StackPanel();
            runtime.Child = runtimeStack;
            runtimeStack.Children.Add(Mute("Runtime", 12));
            runtimeText.Margin = new Thickness(0, 7, 0, 0);
            runtimeText.Foreground = Text;
            runtimeText.TextWrapping = TextWrapping.Wrap;
            runtimeText.FontSize = 12;
            runtimeStack.Children.Add(runtimeText);

            Controls.Grid main = new Controls.Grid();
            main.RowDefinitions.Add(new Controls.RowDefinition { Height = new GridLength(64) });
            main.RowDefinitions.Add(new Controls.RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            main.RowDefinitions.Add(new Controls.RowDefinition { Height = new GridLength(36) });
            Controls.Grid.SetColumn(main, 1);
            root.Children.Add(main);

            Controls.Border header = new Controls.Border { Background = Surface, BorderBrush = Border, BorderThickness = new Thickness(0, 0, 0, 1) };
            main.Children.Add(header);
            Controls.DockPanel headerDock = new Controls.DockPanel { Margin = new Thickness(22, 0, 22, 0) };
            header.Child = headerDock;
            Controls.StackPanel actions = new Controls.StackPanel { Orientation = Controls.Orientation.Horizontal, VerticalAlignment = VerticalAlignment.Center };
            Controls.DockPanel.SetDock(actions, Dock.Right);
            headerDock.Children.Add(actions);
            Controls.Button theme = GhostButton("浅/深");
            theme.Width = 72;
            theme.Click += delegate { dark = !dark; ApplyTheme(); BuildShell(); Navigate(currentView); };
            actions.Children.Add(theme);
            Controls.Button min = GhostButton("最小化");
            min.Width = 82;
            min.Click += delegate { WindowState = WindowState.Minimized; };
            actions.Children.Add(min);
            Controls.StackPanel titleStack = new Controls.StackPanel { VerticalAlignment = VerticalAlignment.Center };
            headerDock.Children.Add(titleStack);
            title.Foreground = Text;
            title.FontSize = 19;
            title.FontWeight = FontWeights.SemiBold;
            titleStack.Children.Add(title);
            titleStack.Children.Add(Mute("Manage Apache, Nginx, MySQL, Redis and MinIO without leaving your desktop.", 12, new Thickness(0, 4, 0, 0)));

            Controls.ScrollViewer scroll = new Controls.ScrollViewer { VerticalScrollBarVisibility = Controls.ScrollBarVisibility.Auto, HorizontalScrollBarVisibility = Controls.ScrollBarVisibility.Disabled };
            Controls.Grid.SetRow(scroll, 1);
            main.Children.Add(scroll);
            content.Margin = new Thickness(22);
            scroll.Content = content;

            Controls.Border footer = new Controls.Border { Background = Surface, BorderBrush = Border, BorderThickness = new Thickness(0, 1, 0, 0) };
            Controls.Grid.SetRow(footer, 2);
            main.Children.Add(footer);
            Controls.DockPanel footerDock = new Controls.DockPanel { Margin = new Thickness(22, 0, 22, 0) };
            footer.Child = footerDock;
            Controls.TextBlock version = Mute("版本：8.1.1.3-local", 12);
            Controls.DockPanel.SetDock(version, Dock.Right);
            footerDock.Children.Add(version);
            statusText.Foreground = Muted;
            statusText.VerticalAlignment = VerticalAlignment.Center;
            footerDock.Children.Add(statusText);
        }

        void AddNav(string id, string label)
        {
            Controls.Button button = new Controls.Button { Content = label, Height = 48, Margin = new Thickness(12, 2, 12, 2), Padding = new Thickness(18, 0, 0, 0), HorizontalContentAlignment = HorizontalAlignment.Left, BorderThickness = new Thickness(0), FontSize = 15 };
            button.Click += delegate { Navigate(id); };
            navPanel.Children.Add(button);
            navButtons[id] = button;
        }

        void StyleNav()
        {
            foreach (KeyValuePair<string, Controls.Button> item in navButtons)
            {
                bool active = item.Key == currentView;
                item.Value.Background = active ? Accent : Media.Brushes.Transparent;
                item.Value.Foreground = active ? Media.Brushes.White : Muted;
            }
        }

        void Navigate(string view)
        {
            currentView = view;
            title.Text = view == "home" ? "首页" : view == "website" ? "网站" : view == "database" ? "数据库" : view == "ftp" ? "FTP" : view == "software" ? "软件管理" : "设置";
            StyleNav();
            content.Children.Clear();
            if (view == "home") Home();
            else if (view == "website") Sites();
            else if (view == "database") Databases();
            else if (view == "ftp") Ftp();
            else if (view == "software") Software();
            else Settings();
            RefreshStatus();
        }

        Controls.TextBlock Txt(string text, double size, FontWeight weight, Thickness margin)
        {
            return new Controls.TextBlock { Text = text, FontSize = size, FontWeight = weight, Foreground = Text, Margin = margin, VerticalAlignment = VerticalAlignment.Center };
        }

        Controls.TextBlock Mute(string text, double size, Thickness? margin = null)
        {
            return new Controls.TextBlock { Text = text, FontSize = size, Foreground = Muted, Margin = margin ?? new Thickness(0), TextWrapping = TextWrapping.Wrap, VerticalAlignment = VerticalAlignment.Center };
        }

        Controls.Border CardBox(double pad = 18)
        {
            return new Controls.Border { Background = Card, BorderBrush = Border, BorderThickness = new Thickness(1), CornerRadius = new CornerRadius(12), Padding = new Thickness(pad) };
        }

        Controls.Button PrimaryButton(string text)
        {
            return new Controls.Button { Content = text, Height = 34, MinWidth = 86, Padding = new Thickness(14, 0, 14, 0), Margin = new Thickness(0, 0, 8, 0), Background = Accent, Foreground = Media.Brushes.White, BorderThickness = new Thickness(0) };
        }

        Controls.Button GhostButton(string text)
        {
            return new Controls.Button { Content = text, Height = 34, MinWidth = 76, Padding = new Thickness(14, 0, 14, 0), Margin = new Thickness(0, 0, 8, 0), Background = Ghost, Foreground = Text, BorderBrush = Border };
        }

        void Home()
        {
            Controls.StackPanel root = new Controls.StackPanel();
            content.Children.Add(root);
            Controls.Border quick = CardBox();
            root.Children.Add(quick);
            Controls.StackPanel quickStack = new Controls.StackPanel();
            quick.Child = quickStack;
            quickStack.Children.Add(Txt("一键启动", 18, FontWeights.SemiBold, new Thickness(0)));
            quickStack.Children.Add(Mute("启动常用服务；退出应用时可选择关闭全部服务或保留后台运行。", 12, new Thickness(0, 6, 0, 0)));
            Controls.StackPanel buttons = new Controls.StackPanel { Orientation = Controls.Orientation.Horizontal, Margin = new Thickness(0, 16, 0, 0) };
            Controls.Button start = PrimaryButton("启动 WAMP/WNMP");
            start.Click += async delegate { await RunOp(delegate { core.StartSuite(); }); };
            Controls.Button stop = GhostButton("停止全部");
            stop.Click += async delegate { await RunOp(delegate { core.StopAllServices(); }); };
            Controls.Button phpmyadmin = GhostButton("打开 phpMyAdmin");
            phpmyadmin.Click += delegate { OpenUrl("http://127.0.0.1/phpmyadmin"); };
            buttons.Children.Add(start); buttons.Children.Add(stop); buttons.Children.Add(phpmyadmin);
            quickStack.Children.Add(buttons);

            Controls.Border svc = CardBox();
            svc.Margin = new Thickness(0, 16, 0, 0);
            root.Children.Add(svc);
            Controls.StackPanel svcStack = new Controls.StackPanel();
            svc.Child = svcStack;
            svcStack.Children.Add(Txt("服务", 18, FontWeights.SemiBold, new Thickness(0)));
            foreach (ServiceInfo service in core.Services) svcStack.Children.Add(ServiceRow(service));

            Controls.Border logs = CardBox();
            logs.Margin = new Thickness(0, 16, 0, 0);
            root.Children.Add(logs);
            Controls.StackPanel logStack = new Controls.StackPanel();
            logs.Child = logStack;
            logStack.Children.Add(Txt("运行日志", 18, FontWeights.SemiBold, new Thickness(0)));
            Controls.TextBox log = new Controls.TextBox { Text = String.Join(Environment.NewLine, core.Config.Logs.Take(100).ToArray()), Margin = new Thickness(0, 14, 0, 0), Height = 145, IsReadOnly = true, Background = Elevated, Foreground = Brush("#6CB6FF"), BorderBrush = Border, FontFamily = new Media.FontFamily("Consolas"), VerticalScrollBarVisibility = Controls.ScrollBarVisibility.Auto, HorizontalScrollBarVisibility = Controls.ScrollBarVisibility.Auto };
            logStack.Children.Add(log);
        }

        UIElement ServiceRow(ServiceInfo s)
        {
            bool running = core.IsRunning(s);
            Controls.Border row = new Controls.Border { Margin = new Thickness(0, 12, 0, 0), Padding = new Thickness(14), CornerRadius = new CornerRadius(10), Background = Elevated };
            Controls.Grid grid = new Controls.Grid();
            grid.ColumnDefinitions.Add(new Controls.ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            grid.ColumnDefinitions.Add(new Controls.ColumnDefinition { Width = new GridLength(110) });
            grid.ColumnDefinitions.Add(new Controls.ColumnDefinition { Width = GridLength.Auto });
            row.Child = grid;
            Controls.StackPanel text = new Controls.StackPanel();
            text.Children.Add(Txt(s.Name, 15, FontWeights.SemiBold, new Thickness(0)));
            text.Children.Add(Mute(File.Exists(s.Exe) ? s.Exe : "未安装：" + s.Exe, 11, new Thickness(0, 3, 0, 0)));
            grid.Children.Add(text);
            Controls.TextBlock state = new Controls.TextBlock { Text = running ? "● 运行中" : (File.Exists(s.Exe) ? "■ 已停止" : "○ 未安装"), Foreground = running ? Good : Muted, VerticalAlignment = VerticalAlignment.Center };
            Controls.Grid.SetColumn(state, 1);
            grid.Children.Add(state);
            Controls.StackPanel ops = new Controls.StackPanel { Orientation = Controls.Orientation.Horizontal };
            Controls.Button toggle = PrimaryButton(running ? "停止" : "启动");
            toggle.Click += async delegate { if (running) await RunOp(delegate { core.StopService(s); }); else await RunOp(delegate { core.StartService(s); }); };
            Controls.Button restart = GhostButton("重启");
            restart.Click += async delegate { await RunOp(delegate { core.RestartService(s); }); };
            Controls.Button cfg = GhostButton("配置");
            cfg.Click += delegate { Navigate("settings"); };
            ops.Children.Add(toggle); ops.Children.Add(restart); ops.Children.Add(cfg);
            Controls.Grid.SetColumn(ops, 2);
            grid.Children.Add(ops);
            return row;
        }

        Controls.StackPanel RootPanel()
        {
            Controls.StackPanel root = new Controls.StackPanel();
            content.Children.Add(root);
            return root;
        }

        Controls.DataGrid Table(object items)
        {
            return new Controls.DataGrid { ItemsSource = items as System.Collections.IEnumerable, AutoGenerateColumns = false, CanUserAddRows = false, IsReadOnly = true, Margin = new Thickness(0, 14, 0, 0), MinHeight = 455, Background = Card, Foreground = Text, BorderBrush = Border, RowBackground = Card, AlternatingRowBackground = Surface, HorizontalGridLinesBrush = Border, GridLinesVisibility = Controls.DataGridGridLinesVisibility.Horizontal };
        }

        Controls.DataGridTextColumn Col(string header, string binding)
        {
            return new Controls.DataGridTextColumn { Header = header, Binding = new Data.Binding(binding), Width = new Controls.DataGridLength(1, Controls.DataGridLengthUnitType.Star) };
        }

        void Sites()
        {
            Controls.StackPanel root = RootPanel();
            Controls.Button create = PrimaryButton("+ 创建网站");
            create.HorizontalAlignment = HorizontalAlignment.Left;
            create.Click += delegate { CreateSite(); };
            root.Children.Add(create);
            Controls.DataGrid grid = Table(core.Config.Sites);
            grid.Columns.Add(Col("网站域名", "Domain")); grid.Columns.Add(Col("端口", "Port")); grid.Columns.Add(Col("物理路径", "Path")); grid.Columns.Add(Col("状态", "Status")); grid.Columns.Add(Col("到期", "Expire"));
            root.Children.Add(grid);
        }

        void Databases()
        {
            Controls.StackPanel root = RootPanel();
            Controls.StackPanel buttons = new Controls.StackPanel { Orientation = Controls.Orientation.Horizontal };
            Controls.Button create = PrimaryButton("+ 创建数据库"); create.Click += delegate { CreateDatabase(); };
            Controls.Button rootPass = GhostButton("修改 root 密码"); rootPass.Click += delegate { ChangeRootPassword(); };
            buttons.Children.Add(create); buttons.Children.Add(rootPass); root.Children.Add(buttons);
            Controls.DataGrid grid = Table(core.Config.Databases);
            grid.Columns.Add(Col("数据库", "Db")); grid.Columns.Add(Col("用户", "User")); grid.Columns.Add(Col("密码", "Pass")); grid.Columns.Add(Col("状态", "Status"));
            root.Children.Add(grid);
        }

        void Ftp()
        {
            Controls.StackPanel root = RootPanel();
            Controls.Button create = PrimaryButton("+ 创建 FTP"); create.HorizontalAlignment = HorizontalAlignment.Left; create.Click += delegate { CreateFtp(); };
            root.Children.Add(create);
            Controls.DataGrid grid = Table(core.Config.FtpAccounts);
            grid.Columns.Add(Col("用户名", "User")); grid.Columns.Add(Col("根目录", "Path")); grid.Columns.Add(Col("权限", "Permission")); grid.Columns.Add(Col("状态", "Status"));
            root.Children.Add(grid);
        }

        void Software()
        {
            Controls.StackPanel root = RootPanel();
            root.Children.Add(Mute("缺失组件可一键下载到 runtime 目录；已有 D:\\phpstudy_pro、D:\\redis-windows-7.2.4、D:\\minio 会自动识别。", 13));
            foreach (SoftwareInfo item in core.Software) root.Children.Add(SoftwareRow(item));
        }

        UIElement SoftwareRow(SoftwareInfo item)
        {
            bool installed = core.IsInstalled(item);
            Controls.Border row = CardBox(14);
            row.Margin = new Thickness(0, 12, 0, 0);
            Controls.Grid grid = new Controls.Grid();
            grid.ColumnDefinitions.Add(new Controls.ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            grid.ColumnDefinitions.Add(new Controls.ColumnDefinition { Width = new GridLength(100) });
            grid.ColumnDefinitions.Add(new Controls.ColumnDefinition { Width = GridLength.Auto });
            row.Child = grid;
            Controls.StackPanel text = new Controls.StackPanel();
            text.Children.Add(Txt(item.Name, 15, FontWeights.SemiBold, new Thickness(0)));
            text.Children.Add(Mute(item.Category + " · " + item.Description + " · " + item.InstallDir, 11, new Thickness(0, 3, 0, 0)));
            grid.Children.Add(text);
            Controls.TextBlock state = new Controls.TextBlock { Text = installed ? "已安装" : "未安装", Foreground = installed ? Good : Muted, VerticalAlignment = VerticalAlignment.Center };
            Controls.Grid.SetColumn(state, 1); grid.Children.Add(state);
            Controls.Button action = installed ? GhostButton("打开目录") : PrimaryButton("安装");
            action.Click += async delegate { if (installed) OpenFolder(item.InstallDir); else await RunOp(delegate { core.InstallSoftware(item, delegate(string msg) { core.AddLog(msg); }); }); };
            Controls.Grid.SetColumn(action, 2); grid.Children.Add(action);
            return row;
        }

        void Settings()
        {
            Controls.Grid root = new Controls.Grid();
            root.RowDefinitions.Add(new Controls.RowDefinition { Height = GridLength.Auto });
            root.RowDefinitions.Add(new Controls.RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            content.Children.Add(root);
            Controls.StackPanel top = new Controls.StackPanel { Orientation = Controls.Orientation.Horizontal };
            Controls.ComboBox combo = new Controls.ComboBox { Width = 240, Height = 34, ItemsSource = core.ConfigFiles, DisplayMemberPath = "Label", SelectedIndex = 0, Margin = new Thickness(0, 0, 10, 0) };
            Controls.Button save = PrimaryButton("保存配置");
            top.Children.Add(combo); top.Children.Add(save); root.Children.Add(top);
            Controls.TextBox editor = new Controls.TextBox { Margin = new Thickness(0, 14, 0, 0), AcceptsReturn = true, AcceptsTab = true, TextWrapping = TextWrapping.NoWrap, VerticalScrollBarVisibility = Controls.ScrollBarVisibility.Auto, HorizontalScrollBarVisibility = Controls.ScrollBarVisibility.Auto, FontFamily = new Media.FontFamily("Consolas"), Background = Card, Foreground = Text, BorderBrush = Border };
            Controls.Grid.SetRow(editor, 1); root.Children.Add(editor);
            Action load = delegate { ConfigFileInfo file = combo.SelectedItem as ConfigFileInfo; if (file != null) editor.Text = core.ReadConfig(file); };
            combo.SelectionChanged += delegate { load(); };
            save.Click += async delegate { ConfigFileInfo file = combo.SelectedItem as ConfigFileInfo; if (file != null) await RunOp(delegate { core.SaveConfig(file, editor.Text); }, false); };
            load();
        }

        async Task RunOp(Action action, bool refresh = true)
        {
            try
            {
                Input.Mouse.OverrideCursor = Input.Cursors.Wait;
                await Task.Factory.StartNew(action);
                if (refresh) Navigate(currentView); else RefreshStatus();
            }
            catch (Exception ex)
            {
                core.AddLog("错误：" + ex.Message);
                MessageBox.Show(this, ex.Message, "操作失败", MessageBoxButton.OK, MessageBoxImage.Error);
            }
            finally
            {
                Input.Mouse.OverrideCursor = null;
            }
        }

        void RefreshStatus()
        {
            string text = String.Join("    ", core.Services.Where(delegate(ServiceInfo s) { return s.Id == "apache" || s.Id == "mysql80" || s.Id == "redis" || s.Id == "minio"; }).Select(delegate(ServiceInfo s) { return (core.IsRunning(s) ? "▶ " : "■ ") + s.Name; }).ToArray());
            runtimeText.Text = text;
            statusText.Text = text;
        }

        void CreateSite()
        {
            FieldDialog d = new FieldDialog("创建网站", new string[] { "域名", "端口", "根目录" }, new string[] { "demo.local", "80", ManagerCore.Slash(Path.Combine(core.Config.WwwRoot, "demo")) });
            d.Owner = this;
            if (d.ShowDialog() == true) { string[] v = d.Values; RunOp(delegate { core.CreateSite(new SiteInfo { Domain = v[0], Port = v[1], Path = v[2] }); }); }
        }

        void CreateDatabase()
        {
            FieldDialog d = new FieldDialog("创建数据库", new string[] { "数据库", "用户", "密码", "root密码(可空)" }, new string[] { "demo_app", "demo_app", "123456", core.Config.MysqlRootPassword }, new bool[] { false, false, true, true });
            d.Owner = this;
            if (d.ShowDialog() == true) { string[] v = d.Values; RunOp(delegate { core.CreateDatabase(v[0], v[1], v[2], v[3]); }); }
        }

        void ChangeRootPassword()
        {
            FieldDialog d = new FieldDialog("修改 root 密码", new string[] { "新密码" }, new string[] { "" }, new bool[] { true });
            d.Owner = this;
            if (d.ShowDialog() == true) RunOp(delegate { core.ChangeRootPassword(d.Values[0]); });
        }

        void CreateFtp()
        {
            FieldDialog d = new FieldDialog("创建 FTP", new string[] { "用户名", "根目录", "权限" }, new string[] { "demo_ftp", ManagerCore.Slash(core.Config.WwwRoot), "读写" });
            d.Owner = this;
            if (d.ShowDialog() == true) { string[] v = d.Values; RunOp(delegate { core.CreateFtp(new FtpInfo { User = v[0], Path = v[1], Permission = v[2] }); }); }
        }

        void OpenFolder(string folder)
        {
            if (!Directory.Exists(folder)) Directory.CreateDirectory(folder);
            Process.Start("explorer.exe", "\"" + folder + "\"");
        }

        void OpenUrl(string url)
        {
            Process.Start(url);
        }

        void BuildTray()
        {
            Forms.ContextMenuStrip menu = new Forms.ContextMenuStrip();
            menu.Items.Add("打开主窗口", null, delegate { ShowFromTray(); });
            menu.Items.Add("一键启动", null, async delegate { await RunOp(delegate { core.StartSuite(); }); });
            menu.Items.Add("停止全部服务", null, async delegate { await RunOp(delegate { core.StopAllServices(); }); });
            menu.Items.Add(new Forms.ToolStripSeparator());
            menu.Items.Add("退出", null, delegate { Close(); });
            tray = new Forms.NotifyIcon { Icon = SystemIcons.Application, Text = "Codex Server Control", Visible = true, ContextMenuStrip = menu };
            tray.DoubleClick += delegate { ShowFromTray(); };
        }

        void ShowFromTray()
        {
            Show();
            WindowState = WindowState.Normal;
            Activate();
        }

        protected override void OnStateChanged(EventArgs e)
        {
            base.OnStateChanged(e);
            if (WindowState == WindowState.Minimized)
            {
                Hide();
                tray.ShowBalloonTip(1200, "Codex Server Control", "应用已最小化到托盘，服务保持运行。", Forms.ToolTipIcon.Info);
            }
        }

        protected override async void OnClosing(System.ComponentModel.CancelEventArgs e)
        {
            if (forceExit)
            {
                tray.Visible = false;
                base.OnClosing(e);
                return;
            }
            e.Cancel = true;
            ExitDialogWpf d = new ExitDialogWpf();
            d.Owner = this;
            if (d.ShowDialog() != true) return;
            if (d.CloseServers) await RunOp(delegate { core.StopAllServices(); }, false);
            forceExit = true;
            tray.Visible = false;
            Close();
        }
    }

    public class FieldDialog : Window
    {
        readonly List<Controls.Control> inputs = new List<Controls.Control>();
        public string[] Values;

        public FieldDialog(string title, string[] labels, string[] defaults, bool[] password = null)
        {
            Title = title; Width = 480; SizeToContent = SizeToContent.Height; WindowStartupLocation = WindowStartupLocation.CenterOwner; ResizeMode = ResizeMode.NoResize; FontFamily = new Media.FontFamily("Microsoft YaHei UI"); Background = new Media.SolidColorBrush(Media.Color.FromRgb(17, 21, 29)); Foreground = Media.Brushes.White;
            Controls.StackPanel root = new Controls.StackPanel { Margin = new Thickness(22) }; Content = root;
            root.Children.Add(new Controls.TextBlock { Text = title, FontSize = 19, FontWeight = FontWeights.SemiBold, Margin = new Thickness(0, 0, 0, 18) });
            for (int i = 0; i < labels.Length; i++)
            {
                root.Children.Add(new Controls.TextBlock { Text = labels[i], Foreground = new Media.SolidColorBrush(Media.Color.FromRgb(148, 163, 184)), Margin = new Thickness(0, 0, 0, 6) });
                if (password != null && password.Length > i && password[i])
                {
                    Controls.PasswordBox box = new Controls.PasswordBox { Height = 32, Margin = new Thickness(0, 0, 0, 12), Password = defaults != null && defaults.Length > i ? defaults[i] : "" };
                    inputs.Add(box); root.Children.Add(box);
                }
                else
                {
                    Controls.TextBox box = new Controls.TextBox { Height = 32, Margin = new Thickness(0, 0, 0, 12), Text = defaults != null && defaults.Length > i ? defaults[i] : "" };
                    inputs.Add(box); root.Children.Add(box);
                }
            }
            Controls.StackPanel buttons = new Controls.StackPanel { Orientation = Controls.Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right, Margin = new Thickness(0, 8, 0, 0) };
            Controls.Button ok = new Controls.Button { Content = "确定", Width = 86, Height = 32, Margin = new Thickness(0, 0, 8, 0), IsDefault = true };
            Controls.Button cancel = new Controls.Button { Content = "取消", Width = 86, Height = 32, IsCancel = true };
            ok.Click += delegate { Values = inputs.Select(delegate(Controls.Control input) { Controls.PasswordBox pb = input as Controls.PasswordBox; return pb != null ? pb.Password : ((Controls.TextBox)input).Text; }).ToArray(); DialogResult = true; };
            buttons.Children.Add(ok); buttons.Children.Add(cancel); root.Children.Add(buttons);
        }
    }

    public class ExitDialogWpf : Window
    {
        public bool CloseServers;
        public ExitDialogWpf()
        {
            Title = "退出应用"; Width = 510; Height = 220; WindowStartupLocation = WindowStartupLocation.CenterOwner; ResizeMode = ResizeMode.NoResize; FontFamily = new Media.FontFamily("Microsoft YaHei UI"); Background = new Media.SolidColorBrush(Media.Color.FromRgb(17, 21, 29)); Foreground = Media.Brushes.White;
            Controls.StackPanel root = new Controls.StackPanel { Margin = new Thickness(24) }; Content = root;
            root.Children.Add(new Controls.TextBlock { Text = "退出时如何处理正在运行的服务？", FontSize = 19, FontWeight = FontWeights.SemiBold });
            root.Children.Add(new Controls.TextBlock { Text = "可以关闭全部 server 服务，也可以保留 MySQL、Redis、MinIO 等在后台继续运行。", Margin = new Thickness(0, 10, 0, 22), TextWrapping = TextWrapping.Wrap, Foreground = new Media.SolidColorBrush(Media.Color.FromRgb(148, 163, 184)) });
            Controls.StackPanel buttons = new Controls.StackPanel { Orientation = Controls.Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right };
            Controls.Button closeAll = new Controls.Button { Content = "关闭全部并退出", Width = 128, Height = 34, Margin = new Thickness(0, 0, 8, 0) };
            closeAll.Click += delegate { CloseServers = true; DialogResult = true; };
            Controls.Button keep = new Controls.Button { Content = "保留后台服务", Width = 118, Height = 34, Margin = new Thickness(0, 0, 8, 0) };
            keep.Click += delegate { CloseServers = false; DialogResult = true; };
            Controls.Button cancel = new Controls.Button { Content = "取消", Width = 86, Height = 34, IsCancel = true };
            buttons.Children.Add(closeAll); buttons.Children.Add(keep); buttons.Children.Add(cancel); root.Children.Add(buttons);
        }
    }
}
