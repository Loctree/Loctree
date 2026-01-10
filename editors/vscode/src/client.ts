/**
 * Language Client for loctree-lsp
 *
 * Sets up the connection between VSCode and the loctree-lsp binary.
 *
 * Created by M&K (c)2025 The LibraxisAI Team
 */

import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
} from 'vscode-languageclient/node';

/**
 * Find the loctree-lsp binary
 *
 * Search order:
 * 1. User-configured path in settings
 * 2. Bundled binary in extension
 * 3. System PATH (cargo install location)
 */
function findServerBinary(context: vscode.ExtensionContext): string {
    const config = vscode.workspace.getConfiguration('loctree');
    const configuredPath = config.get<string>('serverPath');

    if (configuredPath && configuredPath.trim() !== '') {
        return configuredPath;
    }

    // Check for bundled binary
    const bundledPath = path.join(context.extensionPath, 'bin', 'loctree-lsp');
    // For development, just use the command name and let PATH resolve it
    return 'loctree-lsp';
}

/**
 * Create the language client
 */
export function createLanguageClient(
    context: vscode.ExtensionContext,
    outputChannel: vscode.OutputChannel
): LanguageClient {
    const serverCommand = findServerBinary(context);

    outputChannel.appendLine(`Using loctree-lsp binary: ${serverCommand}`);

    const serverOptions: ServerOptions = {
        run: {
            command: serverCommand,
            transport: TransportKind.stdio,
        },
        debug: {
            command: serverCommand,
            transport: TransportKind.stdio,
            args: ['--debug'],
        },
    };

    const clientOptions: LanguageClientOptions = {
        // Register for supported file types
        documentSelector: [
            { scheme: 'file', language: 'typescript' },
            { scheme: 'file', language: 'typescriptreact' },
            { scheme: 'file', language: 'javascript' },
            { scheme: 'file', language: 'javascriptreact' },
            { scheme: 'file', language: 'rust' },
            { scheme: 'file', language: 'python' },
            { scheme: 'file', language: 'go' },
        ],
        synchronize: {
            // Watch for .loctree folder changes
            fileEvents: vscode.workspace.createFileSystemWatcher('**/.loctree/**'),
        },
        outputChannel,
        traceOutputChannel: outputChannel,
    };

    return new LanguageClient(
        'loctree',
        'Loctree Language Server',
        serverOptions,
        clientOptions
    );
}

/**
 * Start the language client
 */
export async function startClient(client: LanguageClient): Promise<void> {
    await client.start();
}

/**
 * Stop the language client
 */
export async function stopClient(client: LanguageClient): Promise<void> {
    if (client.isRunning()) {
        await client.stop();
    }
}
