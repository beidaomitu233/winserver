using Microsoft.UI;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using Windows.Graphics;
using WinRT.Interop;
using Button = Microsoft.UI.Xaml.Controls.Button;
using Forms = System.Windows.Forms;

namespace XpCnLocalManager;

public sealed class MainWindow : Window
{
    sealed class TableRow
    {
        public FrameworkElement[] Cells;
    }

    readonly ManagerCore core;
    readonly Dictionary<string, Button> navButtons = new();
    readonly Dictionary<string, bool> serviceStates = new();
    readonly Grid root = new();
    readonly StackPanel nav = new();
    readonly Grid contentHost = new();
    readonly TextBlock pageTitle = new();
    readonly TextBlock pageSubtitle = new();
    readonly TextBlock statusText = new();
    readonly TextBlock runtimeText = new();
    readonly TextBlock busyText = new();
    readonly Border titleDragRegion = new();
    Forms.NotifyIcon tray;
    AppWindow appWindow;
    bool dark;
    bool forceClose;
    string currentView = "home";
    string softwareCategory = "全部";
    ConfigFileInfo selectedConfig;

    SolidColorBrush Bg, Side, Surface, Card, Elevated, Line, Text, Muted, Ghost, Accent, Good, Bad, Warn, SidebarText, SidebarMuted, SidebarActiveBg, SidebarActiveText;

    public MainWindow()
    {
        Title = "XP.CN 小皮 - 本地开发环境管理";
        core = new ManagerCore(AppRoot());
        core.SyncDatabasesFromMysql();
        selectedConfig = core.ConfigFiles.FirstOrDefault();
        ApplyPalette();
        BuildWindow();
        Navigate("home");
    }

    static string AppRoot()
    {
        DirectoryInfo dir = new(AppContext.BaseDirectory.TrimEnd('\\', '/'));
        for (int i = 0; i < 8 && dir != null; i++, dir = dir.Parent)
        {
            if (Directory.Exists(System.IO.Path.Combine(dir.FullName, "desktop"))) return dir.FullName;
        }
        return AppContext.BaseDirectory;
    }

    SolidColorBrush Brush(string value) => new(Microsoft.UI.ColorHelper.FromArgb(
        255,
        Convert.ToByte(value.Substring(1, 2), 16),
        Convert.ToByte(value.Substring(3, 2), 16),
        Convert.ToByte(value.Substring(5, 2), 16)));

    void ApplyPalette()
    {
        Bg = Brush(dark ? "#0F1117" : "#F4F7FA");
        Side = Brush(dark ? "#121824" : "#1898E2");
        Surface = Brush(dark ? "#111722" : "#FFFFFF");
        Card = Brush(dark ? "#171D29" : "#FFFFFF");
        Elevated = Brush(dark ? "#202838" : "#EDF3F8");
        Line = Brush(dark ? "#29313D" : "#DDE4EE");
        Text = Brush(dark ? "#F2F5F8" : "#151B23");
        Muted = Brush(dark ? "#94A3B8" : "#697789");
        Ghost = Brush(dark ? "#252D3B" : "#EAF1F7");
        Accent = Brush("#1898E2");
        Good = Brush("#35B58F");
        Bad = Brush("#F23838");
        Warn = Brush("#E5A11A");
        SidebarText = Brush("#FFFFFF");
        SidebarMuted = Brush(dark ? "#9FB4CB" : "#DDF2FF");
        SidebarActiveBg = Brush(dark ? "#1898E2" : "#FFFFFF");
        SidebarActiveText = Brush(dark ? "#FFFFFF" : "#1898E2");
    }

    void BuildWindow()
    {
        ExtendsContentIntoTitleBar = true;
        SetTitleBar(titleDragRegion);

        IntPtr hwnd = WindowNative.GetWindowHandle(this);
        WindowId id = Win32Interop.GetWindowIdFromWindow(hwnd);
        appWindow = AppWindow.GetFromWindowId(id);
        appWindow.Resize(new SizeInt32(1180, 780));
        appWindow.Title = "XP.CN 小皮 - 本地开发环境管理";
        appWindow.TitleBar.ButtonBackgroundColor = Colors.Transparent;
        appWindow.TitleBar.ButtonInactiveBackgroundColor = Colors.Transparent;
        appWindow.TitleBar.ButtonForegroundColor = dark ? Colors.White : Colors.Black;
        appWindow.Closing += AppWindow_Closing;

        root.Background = Bg;
        root.RequestedTheme = dark ? ElementTheme.Dark : ElementTheme.Light;
        root.RowDefinitions.Add(new RowDefinition { Height = new GridLength(58) });
        root.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        root.RowDefinitions.Add(new RowDefinition { Height = new GridLength(42) });
        root.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(250) });
        root.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        Content = root;

        BuildTitleBar();
        BuildSidebar();
        BuildContentShell();
        BuildStatusBar();
    }

    void RebuildShell()
    {
        ApplyPalette();
        Detach(titleDragRegion);
        Detach(nav);
        Detach(contentHost);
        Detach(pageTitle);
        Detach(pageSubtitle);
        Detach(busyText);
        Detach(statusText);
        Detach(runtimeText);
        root.Children.Clear();
        nav.Children.Clear();
        navButtons.Clear();
        root.Background = Bg;
        root.RequestedTheme = dark ? ElementTheme.Dark : ElementTheme.Light;
        appWindow.TitleBar.ButtonForegroundColor = dark ? Colors.White : Colors.Black;
        BuildTitleBar();
        BuildSidebar();
        BuildContentShell();
        BuildStatusBar();
        Navigate(currentView);
    }

    void Detach(FrameworkElement element)
    {
        if (element?.Parent is Panel panel)
        {
            panel.Children.Remove(element);
        }
        else if (element?.Parent is Border border && ReferenceEquals(border.Child, element))
        {
            border.Child = null;
        }
        else if (element?.Parent is ContentControl content && ReferenceEquals(content.Content, element))
        {
            content.Content = null;
        }
        else if (element?.Parent is ScrollViewer scroll && ReferenceEquals(scroll.Content, element))
        {
            scroll.Content = null;
        }
    }

    void BuildTitleBar()
    {
        Border bar = new()
        {
            Background = Surface,
            BorderBrush = Line,
            BorderThickness = new Thickness(0, 0, 0, 1)
        };
        Grid.SetRow(bar, 0);
        Grid.SetColumnSpan(bar, 2);
        root.Children.Add(bar);

        Grid grid = new();
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(250) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(245) });
        bar.Child = grid;

        Grid brand = new() { Margin = new Thickness(24, 0, 14, 0), VerticalAlignment = VerticalAlignment.Center };
        brand.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        brand.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        TextBlock xp = new()
        {
            Text = "XP.",
            Foreground = Accent,
            FontSize = 32,
            FontWeight = Microsoft.UI.Text.FontWeights.SemiBold,
            VerticalAlignment = VerticalAlignment.Center
        };
        brand.Children.Add(xp);
        StackPanel sideName = new() { Spacing = 0, VerticalAlignment = VerticalAlignment.Center, Margin = new Thickness(4, 0, 0, 0) };
        sideName.Children.Add(new TextBlock { Text = "小皮", Foreground = Text, FontSize = 14, FontWeight = Microsoft.UI.Text.FontWeights.SemiBold });
        sideName.Children.Add(new TextBlock { Text = "CN", Foreground = Muted, FontSize = 12 });
        Grid.SetColumn(sideName, 1);
        brand.Children.Add(sideName);
        grid.Children.Add(brand);

        titleDragRegion.Background = new SolidColorBrush(Colors.Transparent);
        StackPanel notice = new() { Orientation = Orientation.Horizontal, Spacing = 10, VerticalAlignment = VerticalAlignment.Center, Margin = new Thickness(10, 0, 10, 0) };
        notice.Children.Add(new Border { Width = 8, Height = 8, CornerRadius = new CornerRadius(4), Background = Good, VerticalAlignment = VerticalAlignment.Center });
        notice.Children.Add(Mute("本地 Web 开发环境管理 · Apache / Nginx / MySQL / Redis / MinIO", 13));
        titleDragRegion.Child = notice;
        Grid.SetColumn(titleDragRegion, 1);
        grid.Children.Add(titleDragRegion);

        StackPanel actions = new()
        {
            Orientation = Orientation.Horizontal,
            HorizontalAlignment = HorizontalAlignment.Right,
            VerticalAlignment = VerticalAlignment.Center,
            Spacing = 8,
            Margin = new Thickness(0, 0, 138, 0)
        };
        Button theme = GhostButton(dark ? "浅色" : "深色");
        theme.Click += (_, _) => { dark = !dark; RebuildShell(); };
        Button hide = GhostButton("托盘");
        hide.Click += (_, _) => HideToTray();
        actions.Children.Add(theme);
        actions.Children.Add(hide);
        Grid.SetColumn(actions, 2);
        grid.Children.Add(actions);
    }

    void BuildSidebar()
    {
        Border side = new()
        {
            Background = Side,
            BorderBrush = Line,
            BorderThickness = new Thickness(0, 0, 1, 0)
        };
        Grid.SetRow(side, 1);
        root.Children.Add(side);

        Grid wrap = new();
        wrap.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        wrap.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        side.Child = wrap;

        nav.Spacing = 4;
        nav.Margin = new Thickness(0, 18, 0, 0);
        wrap.Children.Add(nav);
        AddNav("home", "⌂", "首页");
        AddNav("website", "◎", "网站");
        AddNav("database", "◉", "数据库");
        AddNav("ftp", "▣", "FTP");
        AddNav("software", "☷", "软件管理");
        AddNav("settings", "⚙", "设置");

        Border runtime = new()
        {
            Margin = new Thickness(16),
            Padding = new Thickness(14),
            CornerRadius = new CornerRadius(8),
            Background = dark ? Card : Brush("#1688CC"),
            BorderBrush = dark ? Line : Brush("#55B8EF"),
            BorderThickness = new Thickness(1)
        };
        Grid.SetRow(runtime, 1);
        wrap.Children.Add(runtime);
        StackPanel runtimeStack = new() { Spacing = 7 };
        runtime.Child = runtimeStack;
        runtimeStack.Children.Add(new TextBlock { Text = "运行概览", Foreground = SidebarMuted, FontSize = 12 });
        runtimeText.Foreground = SidebarText;
        runtimeText.TextWrapping = TextWrapping.Wrap;
        runtimeText.FontSize = 12;
        runtimeStack.Children.Add(runtimeText);
    }

    void AddNav(string id, string glyph, string text)
    {
        Button button = new()
        {
            Content = glyph + "  " + text,
            Height = 52,
            Margin = new Thickness(12, 0, 12, 0),
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Left,
            Padding = new Thickness(22, 0, 0, 0),
            CornerRadius = new CornerRadius(8),
            BorderThickness = new Thickness(0),
            FontSize = 18,
            FontWeight = Microsoft.UI.Text.FontWeights.SemiBold
        };
        button.Click += (_, _) => Navigate(id);
        nav.Children.Add(button);
        navButtons[id] = button;
    }

    void StyleNav()
    {
        foreach (var item in navButtons)
        {
            bool active = item.Key == currentView;
            item.Value.Background = active ? SidebarActiveBg : new SolidColorBrush(Colors.Transparent);
            item.Value.Foreground = active ? SidebarActiveText : SidebarText;
            item.Value.Opacity = active ? 1 : 0.82;
        }
    }

    void BuildContentShell()
    {
        Grid main = new();
        main.RowDefinitions.Add(new RowDefinition { Height = new GridLength(70) });
        main.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        Grid.SetRow(main, 1);
        Grid.SetColumn(main, 1);
        root.Children.Add(main);

        Grid header = new() { Padding = new Thickness(24, 0, 24, 0) };
        header.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        header.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        StackPanel titleStack = new() { VerticalAlignment = VerticalAlignment.Center, Spacing = 4 };
        pageTitle.Foreground = Text;
        pageTitle.FontSize = 22;
        pageTitle.FontWeight = Microsoft.UI.Text.FontWeights.SemiBold;
        pageSubtitle.Foreground = Muted;
        pageSubtitle.FontSize = 12;
        titleStack.Children.Add(pageTitle);
        titleStack.Children.Add(pageSubtitle);
        header.Children.Add(titleStack);
        busyText.Foreground = Muted;
        busyText.FontSize = 12;
        busyText.HorizontalAlignment = HorizontalAlignment.Right;
        busyText.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(busyText, 1);
        header.Children.Add(busyText);
        main.Children.Add(header);

        ScrollViewer scroll = new()
        {
            VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
            HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled
        };
        contentHost.Padding = new Thickness(24, 0, 24, 24);
        Grid.SetRow(scroll, 1);
        scroll.Content = contentHost;
        main.Children.Add(scroll);
    }

    void BuildStatusBar()
    {
        Border status = new()
        {
            Background = Surface,
            BorderBrush = Line,
            BorderThickness = new Thickness(0, 1, 0, 0),
            Padding = new Thickness(24, 0, 24, 0)
        };
        Grid.SetRow(status, 2);
        Grid.SetColumnSpan(status, 2);
        root.Children.Add(status);

        Grid grid = new();
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        status.Child = grid;
        statusText.Foreground = Muted;
        statusText.FontSize = 12;
        statusText.VerticalAlignment = VerticalAlignment.Center;
        grid.Children.Add(statusText);
        TextBlock version = Mute("版本：8.1.1.3-local", 12);
        version.VerticalAlignment = VerticalAlignment.Center;
        Grid.SetColumn(version, 1);
        grid.Children.Add(version);
    }

    void Navigate(string view)
    {
        currentView = view;
        pageTitle.Text = view switch
        {
            "home" => "首页",
            "website" => "网站",
            "database" => "数据库",
            "ftp" => "FTP",
            "software" => "软件管理",
            _ => "设置"
        };
        pageSubtitle.Text = view switch
        {
            "home" => "一键控制本地 WAMP/WNMP 套件并查看运行状态",
            "website" => "管理站点域名、端口和本地目录",
            "database" => "同步 MySQL 数据库并创建开发账号",
            "ftp" => "维护本地 FTP 账号与站点目录",
            "software" => "安装、识别和打开常用运行组件",
            _ => "编辑 php.ini、httpd.conf、nginx.conf、hosts 等配置文件"
        };
        StyleNav();
        contentHost.Children.Clear();
        if (view == "home") Home();
        else if (view == "website") Sites();
        else if (view == "database") Databases();
        else if (view == "ftp") Ftp();
        else if (view == "software") Software();
        else Settings();
        UpdateStatusText();
    }

    TextBlock Txt(string text, double size = 14, bool bold = false) => new()
    {
        Text = text,
        Foreground = Text,
        FontSize = size,
        FontWeight = bold ? Microsoft.UI.Text.FontWeights.SemiBold : Microsoft.UI.Text.FontWeights.Normal,
        VerticalAlignment = VerticalAlignment.Center,
        TextTrimming = TextTrimming.CharacterEllipsis
    };

    TextBlock Mute(string text, double size = 12) => new()
    {
        Text = text,
        Foreground = Muted,
        FontSize = size,
        TextWrapping = TextWrapping.Wrap,
        VerticalAlignment = VerticalAlignment.Center
    };

    Border CardBox(double pad = 16) => new()
    {
        Background = Card,
        BorderBrush = Line,
        BorderThickness = new Thickness(1),
        CornerRadius = new CornerRadius(8),
        Padding = new Thickness(pad)
    };

    Button PrimaryButton(string text) => new()
    {
        Content = text,
        Height = 34,
        MinWidth = 82,
        Padding = new Thickness(14, 0, 14, 0),
        Background = Accent,
        Foreground = new SolidColorBrush(Colors.White),
        BorderThickness = new Thickness(0),
        CornerRadius = new CornerRadius(6)
    };

    Button GhostButton(string text) => new()
    {
        Content = text,
        Height = 34,
        MinWidth = 74,
        Padding = new Thickness(13, 0, 13, 0),
        Background = Ghost,
        Foreground = Text,
        BorderBrush = Line,
        BorderThickness = new Thickness(1),
        CornerRadius = new CornerRadius(6)
    };

    StackPanel StackRoot()
    {
        StackPanel stack = new() { Spacing = 14 };
        contentHost.Children.Add(stack);
        return stack;
    }

    void Home()
    {
        StackPanel stack = StackRoot();

        Grid metrics = new() { ColumnSpacing = 14 };
        metrics.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        metrics.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        metrics.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        stack.Children.Add(metrics);
        AddMetric(metrics, 0, "运行服务", RunningServiceCount().ToString() + " / " + core.Services.Count, "Apache、MySQL、Redis、MinIO 实时探测");
        AddMetric(metrics, 1, "网站", core.Config.Sites.Count.ToString(), "本地 vhost 与站点目录");
        AddMetric(metrics, 2, "数据库", core.Config.Databases.Count.ToString(), "MySQL8 自动同步可见库");

        Border quick = CardBox();
        StackPanel quickStack = new() { Spacing = 14 };
        quick.Child = quickStack;
        quickStack.Children.Add(Txt("一键控制", 18, true));
        Grid actions = new() { ColumnSpacing = 12 };
        actions.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        actions.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        actions.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        quickStack.Children.Add(actions);
        actions.Children.Add(QuickAction("WAMP / WNMP", SuiteRunning() ? "自动服务正在运行" : "启动 Apache、MySQL 等自动服务", SuiteRunning() ? "停止套件" : "启动套件", async () =>
        {
            if (SuiteRunning()) await RunOp(() => core.StopAllServices());
            else await RunOp(() => core.StartSuite());
        }));
        FrameworkElement php = QuickAction("数据库工具", "打开本机 phpMyAdmin", "打开", () => OpenUrl("http://127.0.0.1/phpmyadmin"));
        Grid.SetColumn(php, 1);
        actions.Children.Add(php);
        FrameworkElement minio = QuickAction("对象存储", "MinIO Console 默认 9001", "控制台", () => OpenUrl("http://127.0.0.1:9001"));
        Grid.SetColumn(minio, 2);
        actions.Children.Add(minio);
        stack.Children.Add(quick);

        Border serviceCard = CardBox();
        StackPanel services = new() { Spacing = 10 };
        serviceCard.Child = services;
        Grid serviceTitle = new();
        serviceTitle.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        serviceTitle.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        serviceTitle.Children.Add(Txt("套件服务", 18, true));
        Button refresh = GhostButton("刷新");
        refresh.Click += async (_, _) => await RefreshServiceStatesAsync(true);
        Grid.SetColumn(refresh, 1);
        serviceTitle.Children.Add(refresh);
        services.Children.Add(serviceTitle);
        foreach (ServiceInfo service in core.Services) services.Children.Add(ServiceRow(service));
        stack.Children.Add(serviceCard);

        Border logs = CardBox();
        StackPanel logStack = new() { Spacing = 10 };
        logs.Child = logStack;
        Grid logHead = new();
        logHead.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        logHead.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        logHead.Children.Add(Txt("运行状态", 18, true));
        Button clear = GhostButton("清空显示");
        clear.Click += (_, _) => { core.Config.Logs.Clear(); core.Save(); Navigate("home"); };
        Grid.SetColumn(clear, 1);
        logHead.Children.Add(clear);
        logStack.Children.Add(logHead);
        ScrollViewer logScroll = new()
        {
            Height = 140,
            Background = Elevated,
            BorderBrush = Line,
            BorderThickness = new Thickness(1),
            Padding = new Thickness(12),
            HorizontalScrollBarVisibility = ScrollBarVisibility.Auto,
            VerticalScrollBarVisibility = ScrollBarVisibility.Auto
        };
        TextBlock log = new()
        {
            Text = string.Join(Environment.NewLine, core.Config.Logs.Take(100)),
            TextWrapping = TextWrapping.NoWrap,
            FontFamily = new Microsoft.UI.Xaml.Media.FontFamily("Consolas"),
            Foreground = Brush("#087DD1")
        };
        logScroll.Content = log;
        logStack.Children.Add(logScroll);
        stack.Children.Add(logs);
    }

    void AddMetric(Grid host, int column, string label, string value, string hint)
    {
        Border card = CardBox(14);
        StackPanel stack = new() { Spacing = 8 };
        card.Child = stack;
        stack.Children.Add(Mute(label, 12));
        stack.Children.Add(Txt(value, 24, true));
        stack.Children.Add(Mute(hint, 11));
        Grid.SetColumn(card, column);
        host.Children.Add(card);
    }

    FrameworkElement QuickAction(string title, string hint, string actionText, Func<Task> action)
    {
        Border card = new()
        {
            Background = Elevated,
            BorderBrush = Line,
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(8),
            Padding = new Thickness(14)
        };
        Grid grid = new();
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        card.Child = grid;
        StackPanel text = new() { Spacing = 4 };
        text.Children.Add(Txt(title, 15, true));
        text.Children.Add(Mute(hint, 11));
        grid.Children.Add(text);
        Button button = PrimaryButton(actionText);
        button.Click += async (_, _) => await action();
        Grid.SetColumn(button, 1);
        grid.Children.Add(button);
        return card;
    }

    FrameworkElement QuickAction(string title, string hint, string actionText, Action action)
    {
        return QuickAction(title, hint, actionText, () =>
        {
            action();
            return Task.CompletedTask;
        });
    }

    UIElement ServiceRow(ServiceInfo service)
    {
        bool running = serviceStates.TryGetValue(service.Id, out bool cached) && cached;
        bool installed = File.Exists(service.Exe);

        Border row = new()
        {
            Background = Elevated,
            CornerRadius = new CornerRadius(8),
            Padding = new Thickness(14)
        };
        ToolTipService.SetToolTip(row, service.Exe);

        Grid grid = new() { ColumnSpacing = 12 };
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(34) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(122) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        row.Child = grid;

        Border marker = new()
        {
            Width = 14,
            Height = 14,
            Background = running ? Good : (installed ? Warn : Bad),
            CornerRadius = new CornerRadius(running ? 7 : 2),
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Center
        };
        grid.Children.Add(marker);

        StackPanel left = new() { Spacing = 3 };
        left.Children.Add(Txt(service.Name, 15, true));
        left.Children.Add(Mute(installed ? ManagerCore.Slash(service.Exe) : "未安装：" + ManagerCore.Slash(service.Exe), 11));
        Grid.SetColumn(left, 1);
        grid.Children.Add(left);

        TextBlock state = new()
        {
            Text = running ? "● 运行中" : (installed ? "■ 已停止" : "○ 未安装"),
            Foreground = running ? Good : Muted,
            VerticalAlignment = VerticalAlignment.Center
        };
        Grid.SetColumn(state, 2);
        grid.Children.Add(state);

        StackPanel ops = new() { Orientation = Orientation.Horizontal, Spacing = 8 };
        Button toggle = running ? GhostButton("停止") : PrimaryButton("启动");
        toggle.Click += async (_, _) =>
        {
            if (running) await RunOp(() => core.StopService(service));
            else await RunOp(() => core.StartService(service));
        };
        Button restart = GhostButton("重启");
        restart.Click += async (_, _) => await RunOp(() => core.RestartService(service));
        Button config = GhostButton("配置");
        config.Click += (_, _) => OpenConfigForPath(service.ConfigFile);
        ops.Children.Add(toggle);
        ops.Children.Add(restart);
        ops.Children.Add(config);
        Grid.SetColumn(ops, 3);
        grid.Children.Add(ops);
        return row;
    }

    void Sites()
    {
        StackPanel stack = StackRoot();
        stack.Children.Add(Toolbar(
            new[]
            {
                ActionButton("+ 创建网站", async () => await CreateSiteAsync(), true),
                ActionButton("打开 WWW", () => OpenFolder(core.Config.WwwRoot), false)
            }));

        IEnumerable<SiteInfo> rows = core.Config.Sites;
        stack.Children.Add(TableCard(
            new[] { "网站域名", "端口", "物理路径", "状态", "到期", "操作" },
            rows.Select(site => new TableRow
            {
                Cells = new FrameworkElement[]
                {
                    Cell(site.Domain, true),
                    Cell(site.Port),
                    Cell(site.Path),
                    StateCell(site.Status),
                    Cell(site.Expire),
                    Actions(
                        RowButton("目录", () => OpenFolder(site.Path)),
                        RowButton("配置", () => OpenConfigForPath(FindConfigPath("vhosts.conf"))))
                }
            }),
            new[] { Star(1.4), Pixel(80), Star(2.4), Pixel(90), Pixel(110), Pixel(150) }));
    }

    void Databases()
    {
        StackPanel stack = StackRoot();
        stack.Children.Add(Toolbar(
            new[]
            {
                ActionButton("+ 创建数据库", async () => await CreateDatabaseAsync(), true),
                ActionButton("修改 root 密码", async () => await ChangeRootPasswordAsync(), false),
                ActionButton("同步", async () => await SyncDatabasesAsync(true), false)
            }));

        IEnumerable<DbInfo> rows = core.Config.Databases;
        stack.Children.Add(TableCard(
            new[] { "数据库", "用户", "密码", "状态", "操作" },
            rows.Select(db => new TableRow
            {
                Cells = new FrameworkElement[]
                {
                    Cell(db.Db, true),
                    Cell(db.User),
                    Cell(db.Pass),
                    StateCell(db.Status),
                    Actions(
                        RowButton("phpMyAdmin", () => OpenUrl("http://127.0.0.1/phpmyadmin")),
                        RowButton("说明", async () => await ShowMessageAsync("数据库操作", "当前 WinUI 版已支持创建与同步数据库。删除或导入导出请先通过 phpMyAdmin / mysql 客户端执行，以避免误删本地数据。")))
                }
            }),
            new[] { Star(1.4), Star(1), Star(1), Pixel(90), Pixel(210) }));
    }

    void Ftp()
    {
        StackPanel stack = StackRoot();
        stack.Children.Add(Toolbar(
            new[]
            {
                ActionButton("+ 创建 FTP", async () => await CreateFtpAsync(), true)
            }));

        IEnumerable<FtpInfo> rows = core.Config.FtpAccounts;
        stack.Children.Add(TableCard(
            new[] { "用户名", "根目录", "权限", "状态", "操作" },
            rows.Select(ftp => new TableRow
            {
                Cells = new FrameworkElement[]
                {
                    Cell(ftp.User, true),
                    Cell(ftp.Path),
                    Cell(ftp.Permission),
                    StateCell(ftp.Status),
                    Actions(RowButton("目录", () => OpenFolder(ftp.Path)))
                }
            }),
            new[] { Star(1), Star(2.2), Pixel(100), Pixel(90), Pixel(110) }));
    }

    StackPanel Toolbar(IEnumerable<Button> leftButtons)
    {
        StackPanel wrap = new() { Orientation = Orientation.Horizontal, Spacing = 8 };
        foreach (Button button in leftButtons) wrap.Children.Add(button);
        return wrap;
    }

    Button ActionButton(string text, Action action, bool primary)
    {
        Button button = primary ? PrimaryButton(text) : GhostButton(text);
        button.Click += (_, _) => action();
        return button;
    }

    Button ActionButton(string text, Func<Task> action, bool primary)
    {
        Button button = primary ? PrimaryButton(text) : GhostButton(text);
        button.Click += async (_, _) => await action();
        return button;
    }

    GridLength Star(double value) => new(value, GridUnitType.Star);
    GridLength Pixel(double value) => new(value);

    Border TableCard(string[] headers, IEnumerable<TableRow> rows, GridLength[] widths)
    {
        Border card = CardBox(0);
        StackPanel stack = new();
        card.Child = stack;

        Grid header = RowGrid(widths);
        header.Height = 40;
        header.Background = Elevated;
        for (int i = 0; i < headers.Length; i++)
        {
            TextBlock h = Mute(headers[i], 12);
            h.FontWeight = Microsoft.UI.Text.FontWeights.SemiBold;
            h.Margin = new Thickness(14, 0, 14, 0);
            Grid.SetColumn(h, i);
            header.Children.Add(h);
        }
        stack.Children.Add(header);

        int count = 0;
        foreach (TableRow row in rows)
        {
            Border border = new()
            {
                BorderBrush = Line,
                BorderThickness = new Thickness(0, 1, 0, 0),
                Background = Card,
                Padding = new Thickness(0, 4, 0, 4),
                MinHeight = 52
            };
            Grid grid = RowGrid(widths);
            border.Child = grid;
            for (int i = 0; i < headers.Length && i < row.Cells.Length; i++)
            {
                FrameworkElement child = row.Cells[i];
                child.Margin = new Thickness(14, 0, 14, 0);
                Grid.SetColumn(row.Cells[i], i);
                grid.Children.Add(row.Cells[i]);
            }
            stack.Children.Add(border);
            count++;
        }

        if (count == 0)
        {
            Border empty = new() { Padding = new Thickness(18), BorderBrush = Line, BorderThickness = new Thickness(0, 1, 0, 0) };
            empty.Child = Mute("暂无数据", 13);
            stack.Children.Add(empty);
        }
        return card;
    }

    Grid RowGrid(GridLength[] widths)
    {
        Grid grid = new() { ColumnSpacing = 0 };
        foreach (GridLength width in widths) grid.ColumnDefinitions.Add(new ColumnDefinition { Width = width });
        return grid;
    }

    FrameworkElement Cell(string text, bool primary = false)
    {
        TextBlock cell = Txt(text ?? "", 13, primary);
        cell.Foreground = primary ? Text : Muted;
        cell.TextTrimming = TextTrimming.CharacterEllipsis;
        cell.VerticalAlignment = VerticalAlignment.Center;
        ToolTipService.SetToolTip(cell, text ?? "");
        return cell;
    }

    FrameworkElement StateCell(string text)
    {
        string value = string.IsNullOrWhiteSpace(text) ? "正常" : text;
        TextBlock cell = Txt(value, 13, false);
        cell.Foreground = value.Contains("正常") ? Good : Warn;
        return cell;
    }

    FrameworkElement Actions(params Button[] buttons)
    {
        StackPanel stack = new() { Orientation = Orientation.Horizontal, Spacing = 8, VerticalAlignment = VerticalAlignment.Center };
        foreach (Button button in buttons) stack.Children.Add(button);
        return stack;
    }

    Button RowButton(string text, Action action)
    {
        Button button = GhostButton(text);
        button.Height = 30;
        button.MinWidth = 62;
        button.Click += (_, _) => action();
        return button;
    }

    Button RowButton(string text, Func<Task> action)
    {
        Button button = GhostButton(text);
        button.Height = 30;
        button.MinWidth = 62;
        button.Click += async (_, _) => await action();
        return button;
    }

    void Software()
    {
        StackPanel stack = StackRoot();
        StackPanel categories = new() { Orientation = Orientation.Horizontal, Spacing = 8 };
        foreach (string category in SoftwareCategories())
        {
            Button button = category == softwareCategory ? PrimaryButton(category) : GhostButton(category);
            button.Click += (_, _) => { softwareCategory = category; Navigate("software"); };
            categories.Children.Add(button);
        }
        stack.Children.Add(categories);
        stack.Children.Add(Mute("缺失组件可下载安装到 runtime 目录；本机已有 D:\\phpstudy_pro、D:\\redis-windows-7.2.4、D:\\minio 时会自动识别。", 13));

        IEnumerable<SoftwareInfo> items = core.Software.Where(item => softwareCategory == "全部" || item.Category == softwareCategory);
        foreach (SoftwareInfo item in items) stack.Children.Add(SoftwareRow(item));
    }

    IEnumerable<string> SoftwareCategories()
    {
        yield return "全部";
        foreach (string category in core.Software.Select(item => item.Category).Distinct(StringComparer.OrdinalIgnoreCase).OrderBy(v => v))
            yield return category;
    }

    UIElement SoftwareRow(SoftwareInfo item)
    {
        bool installed = core.IsInstalled(item);
        Border row = CardBox(14);
        Grid grid = new() { ColumnSpacing = 12 };
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(42) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(100) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        row.Child = grid;

        Border icon = new()
        {
            Width = 34,
            Height = 34,
            CornerRadius = new CornerRadius(8),
            Background = installed ? Good : Elevated,
            BorderBrush = Line,
            BorderThickness = new Thickness(1),
            Child = new TextBlock
            {
                Text = SoftwareGlyph(item),
                Foreground = installed ? new SolidColorBrush(Colors.White) : Muted,
                FontSize = 18,
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Center
            }
        };
        grid.Children.Add(icon);

        StackPanel text = new() { Spacing = 4 };
        text.Children.Add(Txt(item.Name, 15, true));
        text.Children.Add(Mute(item.Category + " · " + item.Description + " · " + ManagerCore.Slash(item.InstallDir), 11));
        Grid.SetColumn(text, 1);
        grid.Children.Add(text);

        TextBlock state = new()
        {
            Text = installed ? "已安装" : (string.IsNullOrWhiteSpace(item.DownloadUrl) ? "需配置" : "未安装"),
            Foreground = installed ? Good : Muted,
            VerticalAlignment = VerticalAlignment.Center
        };
        Grid.SetColumn(state, 2);
        grid.Children.Add(state);

        StackPanel ops = new() { Orientation = Orientation.Horizontal, Spacing = 8 };
        Button action = installed ? GhostButton("目录") : PrimaryButton("安装");
        action.Click += async (_, _) =>
        {
            if (installed) OpenFolder(item.InstallDir);
            else await RunOp(() => core.InstallSoftware(item, msg => core.AddLog(msg)));
        };
        ops.Children.Add(action);
        if (!string.IsNullOrWhiteSpace(item.ServiceId))
        {
            Button config = GhostButton("配置");
            config.Click += (_, _) =>
            {
                ServiceInfo service = core.Services.FirstOrDefault(s => s.Id == item.ServiceId);
                OpenConfigForPath(service?.ConfigFile);
            };
            ops.Children.Add(config);
        }
        Grid.SetColumn(ops, 3);
        grid.Children.Add(ops);
        return row;
    }

    string SoftwareGlyph(SoftwareInfo item)
    {
        string category = item.Category ?? "";
        if (category.Contains("数据库")) return "DB";
        if (category.Contains("redis", StringComparison.OrdinalIgnoreCase)) return "R";
        if (category.Contains("对象")) return "S3";
        if (category.Contains("php", StringComparison.OrdinalIgnoreCase)) return "P";
        return "W";
    }

    void Settings()
    {
        StackPanel stack = StackRoot();
        List<ConfigFileInfo> options = ConfigOptions();
        if (selectedConfig == null) selectedConfig = options.FirstOrDefault();

        ScrollViewer tabsScroll = new()
        {
            HorizontalScrollBarVisibility = ScrollBarVisibility.Auto,
            VerticalScrollBarVisibility = ScrollBarVisibility.Disabled,
            Margin = new Thickness(0, 0, 0, 0)
        };
        StackPanel tabs = new() { Orientation = Orientation.Horizontal, Spacing = 6 };
        tabsScroll.Content = tabs;
        foreach (ConfigFileInfo file in options)
        {
            Button tab = SamePath(file.Path, selectedConfig?.Path) ? PrimaryButton(file.Label) : GhostButton(file.Label);
            tab.Click += (_, _) => { selectedConfig = file; Navigate("settings"); };
            tabs.Children.Add(tab);
        }
        stack.Children.Add(tabsScroll);

        ConfigFileInfo active = selectedConfig ?? options.FirstOrDefault();
        if (active != null)
        {
            Border activeCard = CardBox();
            Grid grid = new() { ColumnSpacing = 12 };
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            activeCard.Child = grid;
            StackPanel text = new() { Spacing = 6 };
            text.Children.Add(Txt(active.Label, 18, true));
            text.Children.Add(Mute(ManagerCore.Slash(active.Path), 12));
            grid.Children.Add(text);
            StackPanel actions = new() { Orientation = Orientation.Horizontal, Spacing = 8, VerticalAlignment = VerticalAlignment.Center };
            actions.Children.Add(RowButton("记事本", () => OpenFileInNotepad(active.Path)));
            actions.Children.Add(RowButton("目录", () => OpenFolder(System.IO.Path.GetDirectoryName(active.Path))));
            Grid.SetColumn(actions, 1);
            grid.Children.Add(actions);
            stack.Children.Add(activeCard);
        }

        stack.Children.Add(TableCard(
            new[] { "配置文件", "路径", "状态", "操作" },
            options.Select(file => new TableRow
            {
                Cells = new FrameworkElement[]
                {
                    Cell(file.Label, SamePath(file.Path, selectedConfig?.Path)),
                    Cell(file.Path),
                    StateCell(File.Exists(file.Path) ? "正常" : "未创建"),
                    Actions(
                        RowButton("选择", () => { selectedConfig = file; Navigate("settings"); }),
                        RowButton("记事本", () => OpenFileInNotepad(file.Path)),
                        RowButton("目录", () => OpenFolder(System.IO.Path.GetDirectoryName(file.Path))))
                }
            }),
            new[] { Pixel(130), Star(2.5), Pixel(90), Pixel(250) }));
    }

    List<ConfigFileInfo> ConfigOptions()
    {
        List<ConfigFileInfo> options = core.ConfigFiles.ToList();
        if (selectedConfig != null && !options.Any(file => SamePath(file.Path, selectedConfig.Path)))
            options.Insert(0, selectedConfig);
        return options;
    }

    string FindConfigPath(string id)
    {
        return core.ConfigFiles.FirstOrDefault(file => string.Equals(file.Id, id, StringComparison.OrdinalIgnoreCase))?.Path;
    }

    void OpenConfigForPath(string filePath)
    {
        if (string.IsNullOrWhiteSpace(filePath))
        {
            Navigate("settings");
            return;
        }
        ConfigFileInfo match = core.ConfigFiles.FirstOrDefault(file => SamePath(file.Path, filePath));
        selectedConfig = match ?? new ConfigFileInfo
        {
            Id = System.IO.Path.GetFileName(filePath),
            Label = System.IO.Path.GetFileName(filePath),
            Path = filePath
        };
        Navigate("settings");
    }

    bool SamePath(string left, string right)
    {
        if (string.IsNullOrWhiteSpace(left) || string.IsNullOrWhiteSpace(right)) return false;
        try
        {
            return string.Equals(System.IO.Path.GetFullPath(left), System.IO.Path.GetFullPath(right), StringComparison.OrdinalIgnoreCase);
        }
        catch
        {
            return string.Equals(left, right, StringComparison.OrdinalIgnoreCase);
        }
    }

    int RunningServiceCount()
    {
        return core.Services.Count(service => serviceStates.TryGetValue(service.Id, out bool running) && running);
    }

    bool SuiteRunning()
    {
        return core.Services.Any(service => service.Auto && serviceStates.TryGetValue(service.Id, out bool running) && running);
    }

    async Task RunOp(Action action, bool refresh = true)
    {
        try
        {
            busyText.Text = "处理中...";
            await Task.Run(action);
            await RefreshServiceStatesAsync(false);
            if (refresh) Navigate(currentView);
            else UpdateStatusText();
        }
        catch (Exception ex)
        {
            core.AddLog("错误：" + ex.Message);
            await ShowMessageAsync("操作失败", ex.Message);
        }
        finally
        {
            busyText.Text = "";
        }
    }

    async Task RefreshServiceStatesAsync(bool rebuild)
    {
        Dictionary<string, bool> latest = await Task.Run(() =>
        {
            Dictionary<string, bool> values = new();
            foreach (ServiceInfo s in core.Services) values[s.Id] = core.IsRunning(s);
            return values;
        });
        await RunOnUiAsync(() =>
        {
            foreach (var item in latest) serviceStates[item.Key] = item.Value;
            UpdateStatusText();
            if (rebuild) Navigate(currentView);
        });
    }

    async Task SyncDatabasesAsync(bool rebuild = false)
    {
        int before = core.Config.Databases.Count;
        await Task.Run(() => core.SyncDatabasesFromMysql());
        if (currentView == "database" && (rebuild || core.Config.Databases.Count != before))
            await RunOnUiAsync(() => Navigate("database"));
    }

    Task RunOnUiAsync(Action action)
    {
        if (DispatcherQueue.HasThreadAccess)
        {
            action();
            return Task.CompletedTask;
        }

        TaskCompletionSource<bool> completion = new();
        if (!DispatcherQueue.TryEnqueue(() =>
        {
            try
            {
                action();
                completion.SetResult(true);
            }
            catch (Exception ex)
            {
                completion.SetException(ex);
            }
        }))
        {
            completion.SetException(new InvalidOperationException("无法切回 UI 线程"));
        }
        return completion.Task;
    }

    void UpdateStatusText()
    {
        string text = string.Join("    ", core.Services
            .Where(s => s.Id is "apache" or "mysql80" or "redis" or "minio")
            .Select(s =>
            {
                bool running = serviceStates.TryGetValue(s.Id, out bool value) && value;
                return (running ? "▶ " : "■ ") + s.Name;
            }));
        statusText.Text = text;
        runtimeText.Text = text;
    }

    async Task CreateSiteAsync()
    {
        string[] values = await FieldDialogAsync("创建网站", new[] { "域名", "端口", "根目录" }, new[] { "demo.local", "80", ManagerCore.Slash(System.IO.Path.Combine(core.Config.WwwRoot, "demo")) });
        if (values == null) return;
        await RunOp(() => core.CreateSite(new SiteInfo { Domain = values[0], Port = values[1], Path = values[2] }));
    }

    async Task CreateDatabaseAsync()
    {
        string[] values = await FieldDialogAsync("创建数据库", new[] { "数据库", "用户", "密码", "root密码(可空)" }, new[] { "demo_app", "demo_app", "123456", core.Config.MysqlRootPassword }, new[] { false, false, true, true });
        if (values == null) return;
        await RunOp(() => core.CreateDatabase(values[0], values[1], values[2], values[3]));
    }

    async Task ChangeRootPasswordAsync()
    {
        string[] values = await FieldDialogAsync("修改 root 密码", new[] { "新密码" }, new[] { "" }, new[] { true });
        if (values == null) return;
        await RunOp(() => core.ChangeRootPassword(values[0]));
    }

    async Task CreateFtpAsync()
    {
        string[] values = await FieldDialogAsync("创建 FTP", new[] { "用户名", "根目录", "权限" }, new[] { "demo_ftp", ManagerCore.Slash(core.Config.WwwRoot), "读写" });
        if (values == null) return;
        await RunOp(() => core.CreateFtp(new FtpInfo { User = values[0], Path = values[1], Permission = values[2] }));
    }

    async Task<string[]> FieldDialogAsync(string title, string[] labels, string[] defaults, bool[] password = null)
    {
        return await Task.FromResult(ShowWinFormsFieldDialog(title, labels, defaults, password));
    }

    string[] ShowWinFormsFieldDialog(string title, string[] labels, string[] defaults, bool[] password)
    {
        using Forms.Form form = new()
        {
            Text = title,
            StartPosition = Forms.FormStartPosition.CenterScreen,
            FormBorderStyle = Forms.FormBorderStyle.FixedDialog,
            MinimizeBox = false,
            MaximizeBox = false,
            ClientSize = new System.Drawing.Size(500, 80 + labels.Length * 42),
            Font = new Font("Microsoft YaHei UI", 10F)
        };

        List<Forms.TextBox> boxes = new();
        for (int i = 0; i < labels.Length; i++)
        {
            Forms.Label label = new()
            {
                Text = labels[i],
                TextAlign = ContentAlignment.MiddleRight
            };
            label.SetBounds(18, 18 + i * 42, 130, 28);
            form.Controls.Add(label);

            Forms.TextBox box = new()
            {
                Text = defaults != null && defaults.Length > i ? defaults[i] : ""
            };
            if (password != null && password.Length > i && password[i]) box.PasswordChar = '*';
            box.SetBounds(160, 18 + i * 42, 300, 28);
            boxes.Add(box);
            form.Controls.Add(box);
        }

        Forms.Button ok = new() { Text = "确定", DialogResult = Forms.DialogResult.OK };
        ok.SetBounds(290, form.ClientSize.Height - 46, 80, 30);
        Forms.Button cancel = new() { Text = "取消", DialogResult = Forms.DialogResult.Cancel };
        cancel.SetBounds(380, form.ClientSize.Height - 46, 80, 30);
        form.Controls.Add(ok);
        form.Controls.Add(cancel);
        form.AcceptButton = ok;
        form.CancelButton = cancel;

        if (form.ShowDialog() != Forms.DialogResult.OK) return null;
        return boxes.Select(box => box.Text).ToArray();
    }

    async Task ShowMessageAsync(string title, string message)
    {
        ContentDialog dialog = new()
        {
            XamlRoot = root.XamlRoot,
            Title = title,
            Content = message,
            CloseButtonText = "知道了"
        };
        await dialog.ShowAsync();
    }

    async void AppWindow_Closing(AppWindow sender, AppWindowClosingEventArgs args)
    {
        if (forceClose)
        {
            if (tray != null) tray.Visible = false;
            return;
        }
        args.Cancel = true;
        ContentDialog dialog = new()
        {
            XamlRoot = root.XamlRoot,
            Title = "退出 XP.CN 小皮",
            Content = "退出时是否关闭所有 server 服务？可以选择保留 MySQL、Redis、MinIO 等继续运行。",
            PrimaryButtonText = "关闭全部并退出",
            SecondaryButtonText = "保留后台服务",
            CloseButtonText = "取消",
            DefaultButton = ContentDialogButton.Secondary
        };
        ContentDialogResult result = await dialog.ShowAsync();
        if (result == ContentDialogResult.None) return;
        if (result == ContentDialogResult.Primary)
            await Task.Run(() => core.StopAllServices());
        forceClose = true;
        if (tray != null) tray.Visible = false;
        Close();
    }

    Forms.NotifyIcon BuildTray()
    {
        Forms.ContextMenuStrip menu = new();
        menu.Items.Add("打开主窗口", null, (_, _) => ShowWindow());
        menu.Items.Add("一键启动", null, async (_, _) => await RunOp(() => core.StartSuite()));
        menu.Items.Add("停止全部服务", null, async (_, _) => await RunOp(() => core.StopAllServices()));
        menu.Items.Add(new Forms.ToolStripSeparator());
        menu.Items.Add("退出", null, (_, _) => Close());
        Forms.NotifyIcon icon = new()
        {
            Icon = SystemIcons.Application,
            Text = "XP.CN 小皮",
            Visible = true,
            ContextMenuStrip = menu
        };
        icon.DoubleClick += (_, _) => ShowWindow();
        return icon;
    }

    void HideToTray()
    {
        tray ??= BuildTray();
        appWindow.Hide();
        tray.ShowBalloonTip(1200, "XP.CN 小皮", "已隐藏到系统托盘，服务保持运行。", Forms.ToolTipIcon.Info);
    }

    void ShowWindow()
    {
        appWindow.Show();
        Activate();
    }

    void OpenFolder(string folder)
    {
        if (string.IsNullOrWhiteSpace(folder)) return;
        if (!Directory.Exists(folder)) Directory.CreateDirectory(folder);
        Process.Start(new ProcessStartInfo("explorer.exe", "\"" + folder + "\"") { UseShellExecute = true });
    }

    void OpenFileInNotepad(string filePath)
    {
        if (string.IsNullOrWhiteSpace(filePath)) return;
        string dir = System.IO.Path.GetDirectoryName(filePath);
        if (!string.IsNullOrWhiteSpace(dir) && !Directory.Exists(dir)) Directory.CreateDirectory(dir);
        if (!File.Exists(filePath)) File.WriteAllText(filePath, "", System.Text.Encoding.UTF8);
        Process.Start(new ProcessStartInfo("notepad.exe", "\"" + filePath + "\"") { UseShellExecute = true });
    }

    void OpenUrl(string url)
    {
        Process.Start(new ProcessStartInfo(url) { UseShellExecute = true });
    }
}
