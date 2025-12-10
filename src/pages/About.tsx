import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getVersion, getName, getTauriVersion } from '@tauri-apps/api/app';
import { open } from '@tauri-apps/plugin-shell';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { ExternalLink, Github, Heart, Coffee } from 'lucide-react';

export function About() {
    const { t } = useTranslation();
    const [appVersion, setAppVersion] = useState('');
    const [appName, setAppName] = useState('');
    const [tauriVersion, setTauriVersion] = useState('');

    useEffect(() => {
        Promise.all([
            getVersion().catch(() => '1.3.0'),
            getName().catch(() => 'VibeHub'),
            getTauriVersion().catch(() => '2.x'),
        ]).then(([version, name, tauri]) => {
            setAppVersion(version);
            setAppName(name);
            setTauriVersion(tauri);
        });
    }, []);

    const openLink = async (url: string) => {
        await open(url);
    };

    return (
        <div className="max-w-2xl mx-auto space-y-6">
            <div>
                <h1 className="text-3xl font-bold tracking-tight">{t('about.title', '关于')}</h1>
                <p className="text-muted-foreground mt-1">{t('about.subtitle', '应用信息和相关链接')}</p>
            </div>

            {/* App Info Card */}
            <Card>
                <CardHeader className="flex flex-row items-center gap-4">
                    <img src="/app-icon.png" alt="VibeHub" className="w-16 h-16 rounded-xl shadow-lg" />
                    <div>
                        <CardTitle className="text-2xl">{appName}</CardTitle>
                        <CardDescription className="flex items-center gap-2 mt-1">
                            <span className="font-mono bg-primary/10 text-primary px-2 py-0.5 rounded">
                                v{appVersion}
                            </span>
                            <span className="text-xs">Portable</span>
                        </CardDescription>
                    </div>
                </CardHeader>
                <CardContent className="space-y-4">
                    <p className="text-muted-foreground">
                        {t('about.description', 'VibeHub 是一个跨平台的开发项目启动器，帮助你快速管理和启动开发项目。')}
                    </p>

                    <div className="grid grid-cols-2 gap-4 text-sm">
                        <div className="space-y-1">
                            <div className="text-muted-foreground">{t('about.tauriVersion', 'Tauri 版本')}</div>
                            <div className="font-mono">{tauriVersion}</div>
                        </div>
                        <div className="space-y-1">
                            <div className="text-muted-foreground">{t('about.license', '开源协议')}</div>
                            <div className="font-mono">Apache-2.0</div>
                        </div>
                    </div>
                </CardContent>
            </Card>

            {/* Links Card */}
            <Card>
                <CardHeader>
                    <CardTitle className="text-lg">{t('about.links', '相关链接')}</CardTitle>
                </CardHeader>
                <CardContent className="space-y-2">
                    <Button
                        variant="outline"
                        className="w-full justify-start"
                        onClick={() => openLink('https://github.com/ChenM0M/VibeHub')}
                    >
                        <Github className="mr-2 h-4 w-4" />
                        GitHub {t('about.repository', '仓库')}
                        <ExternalLink className="ml-auto h-4 w-4 text-muted-foreground" />
                    </Button>
                    <Button
                        variant="outline"
                        className="w-full justify-start"
                        onClick={() => openLink('https://github.com/ChenM0M/VibeHub/releases')}
                    >
                        <ExternalLink className="mr-2 h-4 w-4" />
                        {t('about.releases', '版本发布')}
                        <ExternalLink className="ml-auto h-4 w-4 text-muted-foreground" />
                    </Button>
                    <Button
                        variant="outline"
                        className="w-full justify-start"
                        onClick={() => openLink('https://github.com/ChenM0M/VibeHub/issues')}
                    >
                        <Heart className="mr-2 h-4 w-4" />
                        {t('about.feedback', '问题反馈')}
                        <ExternalLink className="ml-auto h-4 w-4 text-muted-foreground" />
                    </Button>
                </CardContent>
            </Card>

            {/* Credits Card */}
            <Card>
                <CardHeader>
                    <CardTitle className="text-lg">{t('about.credits', '致谢')}</CardTitle>
                </CardHeader>
                <CardContent>
                    <p className="text-sm text-muted-foreground">
                        {t('about.creditsText', '感谢所有开源项目和贡献者，特别是 Tauri、React 和 shadcn/ui 团队。')}
                    </p>
                    <div className="flex items-center gap-2 mt-4 text-xs text-muted-foreground">
                        <Coffee className="h-4 w-4" />
                        <span>{t('about.madeWith', 'Made with ❤️ by the VibeHub team')}</span>
                    </div>
                </CardContent>
            </Card>
        </div>
    );
}
