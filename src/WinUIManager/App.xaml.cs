using Microsoft.UI.Xaml;
using Microsoft.UI.Dispatching;
using System;
using System.Threading;
using WinRT;

namespace XpCnLocalManager;

public partial class App : Microsoft.UI.Xaml.Application
{
    private Window window;

    [STAThread]
    public static void Main()
    {
        ComWrappersSupport.InitializeComWrappers();
        Microsoft.UI.Xaml.Application.Start(_ =>
        {
            SynchronizationContext.SetSynchronizationContext(new DispatcherQueueSynchronizationContext(DispatcherQueue.GetForCurrentThread()));
            new App();
        });
    }

    public App()
    {
        RequestedTheme = ApplicationTheme.Light;
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        window = new MainWindow();
        window.Activate();
    }
}
