/**
 * Command Handlers for Loctree VSCode Extension
 *
 * Registers and implements all loctree commands.
 *
 * Created by M&K (c)2025 The LibraxisAI Team
 */

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

/**
 * Register all loctree commands
 */
export function registerCommands(
    context: vscode.ExtensionContext,
    outputChannel: vscode.OutputChannel,
    workspaceRoot: string
): void {
    // Refresh analysis
    context.subscriptions.push(
        vscode.commands.registerCommand('loctree.refresh', async () => {
            outputChannel.appendLine('Running loctree refresh...');

            const terminal = vscode.window.createTerminal({
                name: 'Loctree',
                cwd: workspaceRoot,
            });

            terminal.show();
            terminal.sendText('loct');

            vscode.window.showInformationMessage(
                'Running loctree analysis. Check the terminal for progress.'
            );
        })
    );

    // Open HTML report
    context.subscriptions.push(
        vscode.commands.registerCommand('loctree.openReport', async (args?: { symbol?: string }) => {
            const reportPath = path.join(workspaceRoot, '.loctree', 'report.html');

            if (!fs.existsSync(reportPath)) {
                const generateReport = await vscode.window.showWarningMessage(
                    'No loctree report found. Generate one now?',
                    'Generate',
                    'Cancel'
                );

                if (generateReport === 'Generate') {
                    const terminal = vscode.window.createTerminal({
                        name: 'Loctree',
                        cwd: workspaceRoot,
                    });
                    terminal.show();
                    terminal.sendText('loct report --open');
                }
                return;
            }

            // Open in browser
            const reportUri = vscode.Uri.file(reportPath);
            await vscode.env.openExternal(reportUri);

            outputChannel.appendLine(`Opened report: ${reportPath}`);
            if (args?.symbol) {
                outputChannel.appendLine(`Navigating to symbol: ${args.symbol}`);
            }
        })
    );

    // Show health summary
    context.subscriptions.push(
        vscode.commands.registerCommand('loctree.showHealth', async () => {
            outputChannel.appendLine('Running loctree health check...');

            const terminal = vscode.window.createTerminal({
                name: 'Loctree Health',
                cwd: workspaceRoot,
            });

            terminal.show();
            terminal.sendText('loct health');
        })
    );

    // Analyze impact
    context.subscriptions.push(
        vscode.commands.registerCommand('loctree.analyzeImpact', async (args?: { file?: string }) => {
            let filePath = args?.file;

            if (!filePath) {
                // Use current editor file
                const editor = vscode.window.activeTextEditor;
                if (editor) {
                    filePath = vscode.workspace.asRelativePath(editor.document.uri);
                }
            }

            if (!filePath) {
                vscode.window.showWarningMessage('No file selected for impact analysis');
                return;
            }

            outputChannel.appendLine(`Analyzing impact for: ${filePath}`);

            const terminal = vscode.window.createTerminal({
                name: 'Loctree Impact',
                cwd: workspaceRoot,
            });

            terminal.show();
            terminal.sendText(`loct impact "${filePath}"`);
        })
    );

    // Find consumers (used by refactoring actions)
    context.subscriptions.push(
        vscode.commands.registerCommand('loctree.findConsumers', async (args?: { file?: string }) => {
            let filePath = args?.file;

            if (!filePath) {
                const editor = vscode.window.activeTextEditor;
                if (editor) {
                    filePath = vscode.workspace.asRelativePath(editor.document.uri);
                }
            }

            if (!filePath) {
                vscode.window.showWarningMessage('No file selected');
                return;
            }

            outputChannel.appendLine(`Finding consumers for: ${filePath}`);

            const terminal = vscode.window.createTerminal({
                name: 'Loctree Consumers',
                cwd: workspaceRoot,
            });

            terminal.show();
            terminal.sendText(`loct query who-imports "${filePath}"`);
        })
    );

    // Show slice (file context)
    context.subscriptions.push(
        vscode.commands.registerCommand('loctree.showSlice', async (args?: { file?: string }) => {
            let filePath = args?.file;

            if (!filePath) {
                const editor = vscode.window.activeTextEditor;
                if (editor) {
                    filePath = vscode.workspace.asRelativePath(editor.document.uri);
                }
            }

            if (!filePath) {
                vscode.window.showWarningMessage('No file selected');
                return;
            }

            outputChannel.appendLine(`Showing slice for: ${filePath}`);

            const terminal = vscode.window.createTerminal({
                name: 'Loctree Slice',
                cwd: workspaceRoot,
            });

            terminal.show();
            terminal.sendText(`loct slice "${filePath}"`);
        })
    );
}
