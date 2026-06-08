using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Input;
using System.Windows.Media;
using Forms = System.Windows.Forms;
using Drawing = System.Drawing;

namespace XpCnLocalManager
{
    public partial class MainWindow : Window
    {
        readonly ManagerCore core;
        readonly Dictionary<string, System.Windows.Controls.Button> navButtons = new();
        Forms.NotifyIcon tray;
        bool dark = true;
        bool forceExit;
        string currentView = "home";

        public MainWindow()
        {
            InitializeComponent();
            core = new ManagerCore(AppRoot());
            ApplyTheme();
            BuildNavigation();
            BuildTray();
            Navigate("home");
        }

        static string AppRoot()
        {
            string dir = AppContext.BaseDirectory.TrimEnd('\\', '/');
            if (Path.GetFileName(dir).Equals("bin", StringComparison.OrdinalIgnoreCase))
                return Directory.GetParent(dir).FullName;
            return dir;
        }

        void ApplyTheme()
        {
            Resources["AppBackgroundBrush"] = Brush(dark ? "#0F1117" : "#F5F7FA");
            Resources["SidebarBrush"] = Brush(dark ? "#161B24" : "#FFFFFF");
            Resources["SurfaceBrush"] = Brush(dark ? "#11151D" : "#FFFFFF");
            Resources["CardBrush"] = Brush(dark ? "#171C26" : "#FFFFFF");
            Resources["ElevatedBrush"] = Brush(dark ? "#1D2430" : "#F0F4F8");
            Resources["BorderBrush"] = Brush(dark ? "#29313D" : "#DDE4EE");
            Resources["TextBrush"] = Brush(dark ? "#F2F5F8" : "#111827");
            Resources["MutedBrush"] = Brush(dark ? "#94A3B8" : "#6B7280");
            Resources["GhostButtonBrush"] = Brush(dark ? "#242B38" : "#E9EEF5");
            Resources["DangerBrush"] = Brush("#EF4444");
            Resources["SuccessBrush"] = Brush("#35C58A");
            ThemeButton.Content = dark ? "浅色" : "深色";
            UpdateNavVisuals();
        }

        SolidColorBrush Brush(string hex)
        {
            return (SolidColorBrush)new BrushConverter().ConvertFromString(hex);
        }

        void BuildNavigation()
        {
            NavPanel.Children.Clear();
            AddNav("home", "⌂  首页");
            AddNav("website", "◎  网站");
            AddNav("database", "◉  数据库");
            AddNav("ftp", "▣  FTP");
            AddNav("software", "☷  软件管理");
            AddNav("settings", "⚙  设置");
        }

        void AddNav(string id, string text)
        {
            Button button = new Button
            {
                Content = text,
                Tag = id,
                Style = (Style)FindResource("NavButtonStyle")
            };
            button.Click += (_, __) => Navigate(id);
            NavPanel.Children.Add(button);
            navButtons[id] = button;
        }

        void UpdateNavVisuals()
        {
            foreach (var item in navButtons)
            {
                bool active = item.Key == currentView;
                item.Value.Background = active ? Brush("#188CE5") : Brushes.Transparent;
                item.Value.Foreground = active ? Brushes.White : (Brush)Resources["MutedBrush"];
            }
        }

        void BuildTray()
        {
            var menu = new Forms.ContextMenuStrip();
            menu.Items.Add("打开主窗口", null, (_, __) => ShowFromTray());
            menu.Items.Add("一键启动", null, async (_, __) => await RunOperation(() => core.StartSuite()));
            menu.Items.Add("停止全部服务", null, async (_, __) => await RunOperation(() => core.StopAllServices()));
            menu.Items.Add(new Forms.ToolStripSeparator());
            menu.Items.Add("退出", null, (_, __) => Close());

            tray = new Forms.NotifyIcon
            {
                Icon = Drawing.SystemIcons.Application,
                Text = "Codex Server Control",
                Visible = true,
                ContextMenuStrip = menu
            };
            tray.DoubleClick += (_, __) => ShowFromTray();
        }

        void Navigate(string view)
        {
            currentView = view;
            PageTitle.Text = view switch
            {
                "home" => "首页",
                "website" => "网站",
                "database" => "数据库",
                "ftp" => "FTP",
                "software" => "软件管理",
                "settings" => "设置",
                _ => "首页"
            };
            UpdateNavVisuals();
            ContentHost.Children.Clear();
            if (view == "home") BuildHome();
            if (view == "website") BuildSites();
            if (view == "database") BuildDatabases();
            if (view == "ftp") BuildFtp();
            if (view == "software") BuildSoftware();
            if (view == "settings") BuildSettings();
            RefreshStatus();
        }

        Border Card(double padding = 18)
        {
            return new Border
            {
                Background = (Brush)Resources["CardBrush"],
                BorderBrush = (Brush)Resources["BorderBrush"],
                BorderThickness = new Thickness(1),
                CornerRadius = new CornerRadius(12),
                Padding = new Thickness(padding)
            };
        }

        TextBlock Label(string text, double size = 14, FontWeight? weight = null)
        {
            return new TextBlock
            {
                Text = text,
                FontSize = size,
                FontWeight = weight ?? FontWeights.Normal,
                Foreground = (Brush)Resources["TextBrush"],
                VerticalAlignment = VerticalAlignment.Center
            };
        }

        TextBlock Muted(string text, double size = 12)
        {
            return new TextBlock
            {
                Text = text,
                FontSize = size,
                Foreground = (Brush)Resources["MutedBrush"],
                VerticalAlignment = VerticalAlignment.Center,
                TextWrapping = TextWrapping.Wrap
            };
        }

        System.Windows.Controls.Button PrimaryButton(string text)
        {
            return new System.Windows.Controls.Button { Content = text, Style = (Style)FindResource("PrimaryButtonStyle"), Margin = new Thickness(0, 0, 8, 0) };
        }

        System.Windows.Controls.Button GhostButton(string text)
        {
            return new System.Windows.Controls.Button { Content = text, Style = (Style)FindResource("GhostButtonStyle"), Margin = new Thickness(0, 0, 8, 0) };
        }

        void BuildHome()
        {
            var root = new Grid();
            root.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            root.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            root.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            root.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            ContentHost.Children.Add(root);

            var quick = Card();
            Grid.SetRow(quick, 0);
            root.Children.Add(quick);
            var quickStack = new StackPanel();
            quick.Child = quickStack;
            quickStack.Children.Add(Label("一键启动", 18, FontWeights.SemiBold));
            quickStack.Children.Add(Muted("启动常用服务；关闭应用时可选择保留 MySQL、Redis、MinIO 等后台服务。"));
            var quickButtons = new StackPanel { Orientation = Orientation.Horizontal, Margin = new Thickness(0, 16, 0, 0) };
            var suite = PrimaryButton("启动 WAMP/WNMP");
            suite.Click += async (_, __) => await RunOperation(() => core.StartSuite());
            var stop = GhostButton("停止全部");
            stop.Click += async (_, __) => await RunOperation(() => core.StopAllServices());
            var dbTool = GhostButton("打开 phpMyAdmin");
            dbTool.Click += (_, __) => OpenUrl("http://127.0.0.1/phpmyadmin");
            quickButtons.Children.Add(suite);
            quickButtons.Children.Add(stop);
            quickButtons.Children.Add(dbTool);
            quickStack.Children.Add(quickButtons);

            var serviceCard = Card();
            serviceCard.Margin = new Thickness(0, 16, 0, 0);
            Grid.SetRow(serviceCard, 1);
            root.Children.Add(serviceCard);
            var serviceStack = new StackPanel();
            serviceCard.Child = serviceStack;
            serviceStack.Children.Add(Label("服务", 18, FontWeights.SemiBold));
            foreach (ServiceInfo service in core.Services)
            {
                serviceStack.Children.Add(ServiceRow(service));
            }

            var logCard = Card();
            logCard.Margin = new Thickness(0, 16, 0, 0);
            Grid.SetRow(logCard, 2);
            root.Children.Add(logCard);
            var logGrid = new Grid();
            logGrid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            logGrid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            logCard.Child = logGrid;
            logGrid.Children.Add(Label("运行日志", 18, FontWeights.SemiBold));
            var log = new TextBox
            {
                Text = string.Join(Environment.NewLine, core.Config.Logs.Take(80)),
                Margin = new Thickness(0, 14, 0, 0),
                Height = 150,
                IsReadOnly = true,
                TextWrapping = TextWrapping.NoWrap,
                VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
                HorizontalScrollBarVisibility = ScrollBarVisibility.Auto,
                Background = (Brush)Resources["ElevatedBrush"],
                Foreground = Brush("#6CB6FF"),
                BorderBrush = (Brush)Resources["BorderBrush"],
                FontFamily = new FontFamily("Consolas")
            };
            Grid.SetRow(log, 1);
            logGrid.Children.Add(log);
        }

        UIElement ServiceRow(ServiceInfo service)
        {
            var border = new Border
            {
                Margin = new Thickness(0, 12, 0, 0),
                Padding = new Thickness(14),
                CornerRadius = new CornerRadius(10),
                Background = (Brush)Resources["ElevatedBrush"]
            };
            var grid = new Grid();
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(110) });
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            border.Child = grid;

            var name = new StackPanel { Orientation = Orientation.Vertical };
            name.Children.Add(Label(service.Name, 15, FontWeights.SemiBold));
            name.Children.Add(Muted(File.Exists(service.Exe) ? service.Exe : "未安装：" + service.Exe, 11));
            grid.Children.Add(name);

            bool running = core.IsRunning(service);
            var status = new TextBlock
            {
                Text = running ? "● 运行中" : (File.Exists(service.Exe) ? "■ 已停止" : "○ 未安装"),
                Foreground = running ? Brush("#35C58A") : (Brush)Resources["MutedBrush"],
                VerticalAlignment = VerticalAlignment.Center
            };
            Grid.SetColumn(status, 1);
            grid.Children.Add(status);

            var buttons = new StackPanel { Orientation = Orientation.Horizontal };
            var toggle = PrimaryButton(running ? "停止" : "启动");
            toggle.Click += async (_, __) =>
            {
                if (running) await RunOperation(() => core.StopService(service));
                else await RunOperation(() => core.StartService(service));
            };
            var restart = GhostButton("重启");
            restart.Click += async (_, __) => await RunOperation(() => core.RestartService(service));
            var config = GhostButton("配置");
            config.Click += (_, __) => OpenConfig(service.ConfigFile);
            buttons.Children.Add(toggle);
            buttons.Children.Add(restart);
            buttons.Children.Add(config);
            Grid.SetColumn(buttons, 2);
            grid.Children.Add(buttons);
            return border;
        }

        void BuildSites()
        {
            var root = VerticalRoot();
            var create = PrimaryButton("+ 创建网站");
            create.HorizontalAlignment = HorizontalAlignment.Left;
            create.Click += (_, __) => CreateSiteDialog();
            root.Children.Add(create);

            var grid = Table(core.Config.Sites);
            grid.Columns.Add(TextColumn("域名", "Domain"));
            grid.Columns.Add(TextColumn("端口", "Port"));
            grid.Columns.Add(TextColumn("物理路径", "Path"));
            grid.Columns.Add(TextColumn("状态", "Status"));
            grid.Columns.Add(TextColumn("到期", "Expire"));
            root.Children.Add(grid);
        }

        void BuildDatabases()
        {
            var root = VerticalRoot();
            var buttons = new StackPanel { Orientation = Orientation.Horizontal };
            var create = PrimaryButton("+ 创建数据库");
            create.Click += (_, __) => CreateDatabaseDialog();
            var rootPass = GhostButton("修改 root 密码");
            rootPass.Click += (_, __) => ChangeRootPasswordDialog();
            buttons.Children.Add(create);
            buttons.Children.Add(rootPass);
            root.Children.Add(buttons);

            var grid = Table(core.Config.Databases);
            grid.Columns.Add(TextColumn("数据库", "Db"));
            grid.Columns.Add(TextColumn("用户", "User"));
            grid.Columns.Add(TextColumn("密码", "Pass"));
            grid.Columns.Add(TextColumn("状态", "Status"));
            root.Children.Add(grid);
        }

        void BuildFtp()
        {
            var root = VerticalRoot();
            var create = PrimaryButton("+ 创建 FTP");
            create.HorizontalAlignment = HorizontalAlignment.Left;
            create.Click += (_, __) => CreateFtpDialog();
            root.Children.Add(create);

            var grid = Table(core.Config.FtpAccounts);
            grid.Columns.Add(TextColumn("用户名", "User"));
            grid.Columns.Add(TextColumn("根目录", "Path"));
            grid.Columns.Add(TextColumn("权限", "Permission"));
            grid.Columns.Add(TextColumn("状态", "Status"));
            root.Children.Add(grid);
        }

        StackPanel VerticalRoot()
        {
            var root = new StackPanel();
            ContentHost.Children.Add(root);
            return root;
        }

        DataGrid Table(object source)
        {
            return new DataGrid
            {
                ItemsSource = source as System.Collections.IEnumerable,
                Margin = new Thickness(0, 14, 0, 0),
                AutoGenerateColumns = false,
                CanUserAddRows = false,
                IsReadOnly = true,
                HeadersVisibility = DataGridHeadersVisibility.Column,
                GridLinesVisibility = DataGridGridLinesVisibility.Horizontal,
                Background = (Brush)Resources["CardBrush"],
                Foreground = (Brush)Resources["TextBrush"],
                BorderBrush = (Brush)Resources["BorderBrush"],
                RowBackground = (Brush)Resources["CardBrush"],
                AlternatingRowBackground = (Brush)Resources["SurfaceBrush"],
                HorizontalGridLinesBrush = (Brush)Resources["BorderBrush"],
                MinHeight = 460
            };
        }

        DataGridTextColumn TextColumn(string header, string binding)
        {
            return new DataGridTextColumn
            {
                Header = header,
                Binding = new Binding(binding),
                Width = new DataGridLength(1, DataGridLengthUnitType.Star)
            };
        }

        void BuildSoftware()
        {
            var root = new StackPanel();
            ContentHost.Children.Add(root);
            root.Children.Add(Muted("缺失组件可一键下载到本应用 runtime 目录；已有 D:\\phpstudy_pro、D:\\redis-windows-7.2.4、D:\\minio 会自动识别。"));

            foreach (SoftwareInfo item in core.Software)
            {
                root.Children.Add(SoftwareRow(item));
            }
        }

        UIElement SoftwareRow(SoftwareInfo item)
        {
            bool installed = core.IsInstalled(item);
            var border = Card(14);
            border.Margin = new Thickness(0, 12, 0, 0);
            var grid = new Grid();
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(120) });
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            border.Child = grid;

            var text = new StackPanel();
            text.Children.Add(Label(item.Name, 15, FontWeights.SemiBold));
            text.Children.Add(Muted(item.Category + " · " + item.Description + " · " + item.InstallDir, 11));
            grid.Children.Add(text);

            var state = new TextBlock
            {
                Text = installed ? "已安装" : "未安装",
                Foreground = installed ? Brush("#35C58A") : (Brush)Resources["MutedBrush"],
                VerticalAlignment = VerticalAlignment.Center
            };
            Grid.SetColumn(state, 1);
            grid.Children.Add(state);

            var buttons = new StackPanel { Orientation = Orientation.Horizontal };
            var action = installed ? GhostButton("打开目录") : PrimaryButton("安装");
            action.Click += async (_, __) =>
            {
                if (installed) OpenFolder(item.InstallDir);
                else await RunOperation(() => core.InstallSoftware(item, msg => core.AddLog(msg)));
            };
            buttons.Children.Add(action);
            Grid.SetColumn(buttons, 2);
            grid.Children.Add(buttons);
            return border;
        }

        void BuildSettings()
        {
            var root = new Grid();
            root.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            root.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            ContentHost.Children.Add(root);

            var top = new StackPanel { Orientation = Orientation.Horizontal };
            var combo = new ComboBox
            {
                Width = 240,
                Height = 34,
                ItemsSource = core.ConfigFiles,
                DisplayMemberPath = "Label",
                SelectedIndex = 0,
                Margin = new Thickness(0, 0, 10, 0)
            };
            var save = PrimaryButton("保存配置");
            top.Children.Add(combo);
            top.Children.Add(save);
            root.Children.Add(top);

            var editor = new TextBox
            {
                Margin = new Thickness(0, 14, 0, 0),
                AcceptsReturn = true,
                AcceptsTab = true,
                TextWrapping = TextWrapping.NoWrap,
                VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
                HorizontalScrollBarVisibility = ScrollBarVisibility.Auto,
                FontFamily = new FontFamily("Consolas"),
                Background = (Brush)Resources["CardBrush"],
                Foreground = (Brush)Resources["TextBrush"],
                BorderBrush = (Brush)Resources["BorderBrush"]
            };
            Grid.SetRow(editor, 1);
            root.Children.Add(editor);

            Action load = () =>
            {
                if (combo.SelectedItem is ConfigFileInfo file)
                    editor.Text = core.ReadConfig(file);
            };
            combo.SelectionChanged += (_, __) => load();
            save.Click += async (_, __) =>
            {
                if (combo.SelectedItem is ConfigFileInfo file)
                    await RunOperation(() => core.SaveConfig(file, editor.Text), false);
            };
            load();
        }

        async Task RunOperation(Action action, bool refresh = true)
        {
            try
            {
                Mouse.OverrideCursor = Cursors.Wait;
                await Task.Run(action);
                if (refresh) Navigate(currentView);
                else RefreshStatus();
            }
            catch (Exception ex)
            {
                core.AddLog("错误：" + ex.Message);
                MessageBox.Show(this, ex.Message, "操作失败", MessageBoxButton.OK, MessageBoxImage.Error);
            }
            finally
            {
                Mouse.OverrideCursor = null;
            }
        }

        void RefreshStatus()
        {
            var parts = core.Services
                .Where(s => s.Id is "apache" or "mysql80" or "redis" or "minio")
                .Select(s => (core.IsRunning(s) ? "▶ " : "■ ") + s.Name);
            string text = string.Join("    ", parts);
            RuntimeText.Text = text;
            StatusText.Text = text;
        }

        void CreateSiteDialog()
        {
            var dialog = new FieldDialog("创建网站",
                new[] { "域名", "端口", "根目录" },
                new[] { "demo.local", "80", ManagerCore.Slash(Path.Combine(core.Config.WwwRoot, "demo")) });
            if (dialog.ShowDialog() != true) return;
            var v = dialog.Values;
            _ = RunOperation(() => core.CreateSite(new SiteInfo { Domain = v[0], Port = v[1], Path = v[2] }));
        }

        void CreateDatabaseDialog()
        {
            var dialog = new FieldDialog("创建数据库",
                new[] { "数据库", "用户", "密码", "root密码(可空)" },
                new[] { "demo_app", "demo_app", "123456", core.Config.MysqlRootPassword },
                new[] { false, false, true, true });
            if (dialog.ShowDialog() != true) return;
            var v = dialog.Values;
            _ = RunOperation(() => core.CreateDatabase(v[0], v[1], v[2], v[3]));
        }

        void ChangeRootPasswordDialog()
        {
            var dialog = new FieldDialog("修改 root 密码", new[] { "新密码" }, new[] { "" }, new[] { true });
            if (dialog.ShowDialog() != true) return;
            _ = RunOperation(() => core.ChangeRootPassword(dialog.Values[0]));
        }

        void CreateFtpDialog()
        {
            var dialog = new FieldDialog("创建 FTP",
                new[] { "用户名", "根目录", "权限" },
                new[] { "demo_ftp", ManagerCore.Slash(core.Config.WwwRoot), "读写" });
            if (dialog.ShowDialog() != true) return;
            var v = dialog.Values;
            _ = RunOperation(() => core.CreateFtp(new FtpInfo { User = v[0], Path = v[1], Permission = v[2] }));
        }

        void OpenConfig(string filePath)
        {
            Navigate("settings");
        }

        void OpenFolder(string folder)
        {
            if (!Directory.Exists(folder)) Directory.CreateDirectory(folder);
            Process.Start(new ProcessStartInfo("explorer.exe", "\"" + folder + "\"") { UseShellExecute = true });
        }

        void OpenUrl(string url)
        {
            Process.Start(new ProcessStartInfo(url) { UseShellExecute = true });
        }

        void ShowFromTray()
        {
            Show();
            WindowState = WindowState.Normal;
            Activate();
        }

        void ThemeButton_Click(object sender, RoutedEventArgs e)
        {
            dark = !dark;
            ApplyTheme();
            Navigate(currentView);
        }

        void MinimizeButton_Click(object sender, RoutedEventArgs e)
        {
            WindowState = WindowState.Minimized;
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
            var dialog = new ExitChoiceDialog { Owner = this };
            if (dialog.ShowDialog() != true) return;
            if (dialog.CloseServers)
            {
                await RunOperation(() => core.StopAllServices(), false);
            }
            forceExit = true;
            tray.Visible = false;
            Close();
        }
    }

    public class FieldDialog : Window
    {
        readonly List<System.Windows.Controls.Control> inputs = new();
        public string[] Values { get; private set; }

        public FieldDialog(string title, string[] labels, string[] defaults, bool[] password = null)
        {
            Title = title;
            Width = 480;
            SizeToContent = SizeToContent.Height;
            WindowStartupLocation = WindowStartupLocation.CenterOwner;
            ResizeMode = ResizeMode.NoResize;
            Background = new SolidColorBrush(Color.FromRgb(17, 21, 29));
            Foreground = Brushes.White;
            FontFamily = new FontFamily("Microsoft YaHei UI");

            var root = new StackPanel { Margin = new Thickness(22) };
            Content = root;
            root.Children.Add(new TextBlock { Text = title, FontSize = 19, FontWeight = FontWeights.SemiBold, Margin = new Thickness(0, 0, 0, 18) });

            for (int i = 0; i < labels.Length; i++)
            {
                root.Children.Add(new TextBlock { Text = labels[i], Foreground = new SolidColorBrush(Color.FromRgb(148, 163, 184)), Margin = new Thickness(0, 0, 0, 6) });
                if (password != null && i < password.Length && password[i])
                {
                    var box = new PasswordBox { Height = 32, Margin = new Thickness(0, 0, 0, 12) };
                    box.Password = defaults != null && i < defaults.Length ? defaults[i] : "";
                    inputs.Add(box);
                    root.Children.Add(box);
                }
                else
                {
                    var box = new TextBox { Height = 32, Text = defaults != null && i < defaults.Length ? defaults[i] : "", Margin = new Thickness(0, 0, 0, 12) };
                    inputs.Add(box);
                    root.Children.Add(box);
                }
            }

            var buttons = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right, Margin = new Thickness(0, 8, 0, 0) };
            var ok = new Button { Content = "确定", Width = 86, Height = 32, Margin = new Thickness(0, 0, 8, 0), IsDefault = true };
            var cancel = new Button { Content = "取消", Width = 86, Height = 32, IsCancel = true };
            ok.Click += (_, __) =>
            {
                Values = inputs.Select(input => input is PasswordBox pb ? pb.Password : ((TextBox)input).Text).ToArray();
                DialogResult = true;
            };
            buttons.Children.Add(ok);
            buttons.Children.Add(cancel);
            root.Children.Add(buttons);
        }
    }

    public class ExitChoiceDialog : Window
    {
        public bool CloseServers { get; private set; }

        public ExitChoiceDialog()
        {
            Title = "退出应用";
            Width = 500;
            Height = 210;
            WindowStartupLocation = WindowStartupLocation.CenterOwner;
            ResizeMode = ResizeMode.NoResize;
            Background = new SolidColorBrush(Color.FromRgb(17, 21, 29));
            Foreground = Brushes.White;
            FontFamily = new FontFamily("Microsoft YaHei UI");

            var root = new StackPanel { Margin = new Thickness(24) };
            Content = root;
            root.Children.Add(new TextBlock { Text = "退出时如何处理正在运行的服务？", FontSize = 19, FontWeight = FontWeights.SemiBold });
            root.Children.Add(new TextBlock
            {
                Text = "可以关闭全部 server 服务，也可以保留 MySQL、Redis、MinIO 等在后台继续运行。",
                Margin = new Thickness(0, 10, 0, 22),
                TextWrapping = TextWrapping.Wrap,
                Foreground = new SolidColorBrush(Color.FromRgb(148, 163, 184))
            });

            var buttons = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right };
            var closeAll = new Button { Content = "关闭全部并退出", Width = 128, Height = 34, Margin = new Thickness(0, 0, 8, 0) };
            closeAll.Click += (_, __) => { CloseServers = true; DialogResult = true; };
            var keep = new Button { Content = "保留后台服务", Width = 118, Height = 34, Margin = new Thickness(0, 0, 8, 0) };
            keep.Click += (_, __) => { CloseServers = false; DialogResult = true; };
            var cancel = new Button { Content = "取消", Width = 86, Height = 34, IsCancel = true };
            buttons.Children.Add(closeAll);
            buttons.Children.Add(keep);
            buttons.Children.Add(cancel);
            root.Children.Add(buttons);
        }
    }
}
