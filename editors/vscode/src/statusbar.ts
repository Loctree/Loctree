/**
 * Status Bar for Loctree
 *
 * Shows loctree health status in the VSCode status bar.
 *
 * Created by M&K (c)2025 The LibraxisAI Team
 */

import * as vscode from 'vscode';

export enum StatusBarState {
    Initializing = 'initializing',
    Ready = 'ready',
    Analyzing = 'analyzing',
    HasIssues = 'hasIssues',
    Healthy = 'healthy',
    Error = 'error',
    Stopped = 'stopped',
}

export interface StatusBarData {
    deadCount: number;
    cycleCount: number;
    twinCount: number;
}

/**
 * Create the loctree status bar item
 */
export function createStatusBarItem(): vscode.StatusBarItem {
    const item = vscode.window.createStatusBarItem(
        vscode.StatusBarAlignment.Right,
        100
    );

    item.name = 'Loctree';
    item.command = 'loctree.showHealth';
    item.tooltip = 'Click to show loctree health summary';

    return item;
}

/**
 * Update status bar with current state
 */
export function updateStatusBar(
    item: vscode.StatusBarItem,
    state: StatusBarState,
    data?: StatusBarData
): void {
    switch (state) {
        case StatusBarState.Initializing:
            item.text = '$(loading~spin) Loctree';
            item.tooltip = 'Loctree is initializing...';
            item.backgroundColor = undefined;
            break;

        case StatusBarState.Ready:
            item.text = '$(tree) Loctree';
            item.tooltip = 'Loctree is ready';
            item.backgroundColor = undefined;
            break;

        case StatusBarState.Analyzing:
            item.text = '$(sync~spin) Loctree';
            item.tooltip = 'Analyzing codebase...';
            item.backgroundColor = undefined;
            break;

        case StatusBarState.HasIssues:
            if (data) {
                const issues = [];
                if (data.deadCount > 0) issues.push(`${data.deadCount} dead`);
                if (data.cycleCount > 0) issues.push(`${data.cycleCount} cycles`);
                if (data.twinCount > 0) issues.push(`${data.twinCount} twins`);

                item.text = `$(warning) Loctree: ${issues.join(', ')}`;
                item.tooltip = `Click to see details\n${issues.join('\n')}`;
                item.backgroundColor = new vscode.ThemeColor(
                    'statusBarItem.warningBackground'
                );
            } else {
                item.text = '$(warning) Loctree: issues found';
                item.tooltip = 'Click to see loctree issues';
                item.backgroundColor = new vscode.ThemeColor(
                    'statusBarItem.warningBackground'
                );
            }
            break;

        case StatusBarState.Healthy:
            item.text = '$(check) Loctree: healthy';
            item.tooltip = 'No dead code or cycles detected';
            item.backgroundColor = undefined;
            break;

        case StatusBarState.Error:
            item.text = '$(error) Loctree';
            item.tooltip = 'Loctree encountered an error. Click to see details.';
            item.backgroundColor = new vscode.ThemeColor(
                'statusBarItem.errorBackground'
            );
            break;

        case StatusBarState.Stopped:
            item.text = '$(debug-stop) Loctree';
            item.tooltip = 'Loctree server stopped';
            item.backgroundColor = undefined;
            break;
    }

    item.show();
}

/**
 * Update status bar with diagnostic counts
 */
export function updateStatusBarWithCounts(
    item: vscode.StatusBarItem,
    deadCount: number,
    cycleCount: number,
    twinCount: number
): void {
    const totalIssues = deadCount + cycleCount + twinCount;

    if (totalIssues === 0) {
        updateStatusBar(item, StatusBarState.Healthy);
    } else {
        updateStatusBar(item, StatusBarState.HasIssues, {
            deadCount,
            cycleCount,
            twinCount,
        });
    }
}
