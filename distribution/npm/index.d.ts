/**
 * TypeScript definitions for loctree npm package
 */

import { ExecFileSyncOptions } from 'child_process';

/**
 * Execute loct with given arguments
 * @param args - Command line arguments to pass to loct
 * @param options - Node.js child_process execution options
 * @returns stdout from loct execution
 * @throws Error if loct binary is not found or execution fails
 */
export function execLoctree(args?: string[], options?: ExecFileSyncOptions): Buffer;

/**
 * Execute loct and return result as string
 * @param args - Command line arguments to pass to loct
 * @returns stdout from loct as UTF-8 string
 * @throws Error if loct binary is not found or execution fails
 */
export function execLoctreeSync(args?: string[]): string;

/**
 * Get the absolute path to the loct binary for the current platform
 * @returns Absolute path to the loct binary
 * @throws Error if platform is unsupported or binary is not found
 */
export function getBinaryPath(): string;
