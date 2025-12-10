import { useState, useEffect } from 'react';
import { tauriApi } from '@/services/tauri';
import { open } from '@tauri-apps/plugin-shell';
import { useTranslation } from 'react-i18next';
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
    DialogDescription,
    DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Download, ExternalLink, X } from 'lucide-react';

interface UpdateInfo {
    has_update: boolean;
    current_version: string;
    latest_version: string;
    release_notes: string | null;
    release_url: string | null;
    download_url: string | null;
}

interface UpdateCheckerProps {
    showManualCheckResult?: boolean;
    onManualCheckComplete?: () => void;
}

export function UpdateChecker({ showManualCheckResult, onManualCheckComplete }: UpdateCheckerProps) {
    const { t } = useTranslation();
    const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
    const [isOpen, setIsOpen] = useState(false);
    const [isChecking, setIsChecking] = useState(false);
    const [noUpdateMessage, setNoUpdateMessage] = useState(false);

    // Only check when manually triggered
    useEffect(() => {
        if (showManualCheckResult) {
            checkUpdate(true);
        }
    }, [showManualCheckResult]);

    const checkUpdate = async (isManual: boolean) => {
        if (isChecking) return;
        setIsChecking(true);
        setNoUpdateMessage(false);

        try {
            const result = await tauriApi.checkForUpdates();
            if (result.has_update) {
                setUpdateInfo(result);
                setIsOpen(true);
            } else if (isManual) {
                setNoUpdateMessage(true);
                setTimeout(() => {
                    setNoUpdateMessage(false);
                    onManualCheckComplete?.();
                }, 2000);
            }
        } catch (error) {
            console.error('Update check failed:', error);
            if (isManual) {
                onManualCheckComplete?.();
            }
        } finally {
            setIsChecking(false);
        }
    };

    const handleDownload = async () => {
        if (updateInfo?.release_url) {
            await open(updateInfo.release_url);
            setIsOpen(false);
        }
    };

    const handleClose = () => {
        setIsOpen(false);
        onManualCheckComplete?.();
    };

    // Show "no update" toast-like message
    if (noUpdateMessage) {
        return (
            <div className="fixed bottom-4 right-4 z-50 animate-in slide-in-from-bottom-2">
                <div className="bg-card border rounded-lg shadow-lg px-4 py-3 flex items-center gap-2">
                    <span className="text-green-500">✓</span>
                    <span className="text-sm">{t('update.noUpdate', '已是最新版本')}</span>
                </div>
            </div>
        );
    }

    if (!updateInfo) return null;

    return (
        <Dialog open={isOpen} onOpenChange={handleClose}>
            <DialogContent className="max-w-md">
                <DialogHeader>
                    <DialogTitle className="flex items-center gap-2">
                        <Download className="h-5 w-5 text-primary" />
                        {t('update.newVersion', '发现新版本')}
                    </DialogTitle>
                    <DialogDescription className="flex items-center gap-2">
                        <span className="font-mono bg-muted px-2 py-0.5 rounded">
                            v{updateInfo.current_version}
                        </span>
                        <span>→</span>
                        <span className="font-mono bg-primary/10 text-primary px-2 py-0.5 rounded">
                            v{updateInfo.latest_version}
                        </span>
                    </DialogDescription>
                </DialogHeader>

                {updateInfo.release_notes && (
                    <div className="max-h-48 overflow-y-auto rounded-md bg-muted p-3 text-sm">
                        <pre className="whitespace-pre-wrap font-sans text-muted-foreground">
                            {updateInfo.release_notes}
                        </pre>
                    </div>
                )}

                <DialogFooter className="flex gap-2 sm:justify-end">
                    <Button variant="outline" onClick={handleClose}>
                        <X className="mr-2 h-4 w-4" />
                        {t('update.later', '稍后提醒')}
                    </Button>
                    <Button onClick={handleDownload}>
                        <ExternalLink className="mr-2 h-4 w-4" />
                        {t('update.download', '前往下载')}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}
