/**
 * Loctree VSCode Extension
 *
 * Provides IDE integration for dead code detection, circular import analysis,
 * and codebase navigation powered by the loctree-lsp server.
 *
 * Created by M&K (c)2025 The LibraxisAI Team
 */

import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import { createLanguageClient, startClient, stopClient } from './client';
import { createStatusBarItem, updateStatusBar, StatusBarState } from './statusbar';
import { registerCommands } from './commands';

let client: LanguageClient | undefined;
let statusBarItem: vscode.StatusBarItem | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
    const outputChannel = vscode.window.createOutputChannel('Loctree');
    outputChannel.appendLine('Loctree extension activating...');

    // Check if .loctree folder exists
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
        outputChannel.appendLine('No workspace folder found');
        return;
    }

    const workspaceRoot = workspaceFolders[0].uri.fsPath;
    const loctreeFolder = vscode.Uri.joinPath(workspaceFolders[0].uri, '.loctree');

    try {
        await vscode.workspace.fs.stat(loctreeFolder);
        outputChannel.appendLine('.loctree folder found, starting LSP client');
    } catch {
        outputChannel.appendLine('.loctree folder not found. Run `loct` to scan your project first.');
        vscode.window.showInformationMessage(
            'Loctree: No .loctree folder found. Run `loct` in your terminal to scan the project.'
        );
        // Still register commands so user can run refresh
    }

    // Create status bar item
    const config = vscode.workspace.getConfiguration('loctree');
    if (config.get<boolean>('showStatusBar', true)) {
        statusBarItem = createStatusBarItem();
        context.subscriptions.push(statusBarItem);
        updateStatusBar(statusBarItem, StatusBarState.Initializing);
    }

    // Register commands
    registerCommands(context, outputChannel, workspaceRoot);

    // Create and start language client
    try {
        client = createLanguageClient(context, outputChannel);
        await startClient(client);
        outputChannel.appendLine('Loctree LSP client started successfully');

        if (statusBarItem) {
            updateStatusBar(statusBarItem, StatusBarState.Ready);
        }

        // Listen for diagnostics to update status bar
        client.onDidChangeState((event) => {
            if (statusBarItem) {
                if (event.newState === 1) { // Stopped
                    updateStatusBar(statusBarItem, StatusBarState.Stopped);
                } else if (event.newState === 2) { // Running
                    updateStatusBar(statusBarItem, StatusBarState.Ready);
                }
            }
        });
    } catch (error) {
        outputChannel.appendLine(`Failed to start LSP client: ${error}`);
        if (statusBarItem) {
            updateStatusBar(statusBarItem, StatusBarState.Error);
        }
    }

    // Watch for .loctree/snapshot.json changes
    const snapshotWatcher = vscode.workspace.createFileSystemWatcher(
        new vscode.RelativePattern(workspaceFolders[0], '.loctree/snapshot.json')
    );

    snapshotWatcher.onDidChange(() => {
        outputChannel.appendLine('Snapshot changed, refreshing diagnostics...');
        if (client) {
            // Send notification to refresh
            client.sendNotification('loctree/refresh');
        }
    });

    context.subscriptions.push(snapshotWatcher);
    context.subscriptions.push(outputChannel);

    outputChannel.appendLine('Loctree extension activated');
}

export async function deactivate(): Promise<void> {
    if (client) {
        await stopClient(client);
    }
}
