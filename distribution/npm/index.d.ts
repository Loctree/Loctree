/**
 * TypeScript definitions for loctree npm package
 */

import { ExecFileSyncOptions } from 'child_process';

/**
 * Execute loctree with given arguments
 * @param args - Command line arguments to pass to loctree
 * @param options - Node.js child_process execution options
 * @returns stdout from loctree execution
 * @throws Error if loctree binary is not found or execution fails
 */
export function execLoctree(args?: string[], options?: ExecFileSyncOptions): Buffer;

/**
 * Execute loctree and return result as string
 * @param args - Command line arguments to pass to loctree
 * @returns stdout from loctree as UTF-8 string
 * @throws Error if loctree binary is not found or execution fails
 */
export function execLoctreeSync(args?: string[]): string;

/**
 * Get the absolute path to the loctree binary for the current platform
 * @returns Absolute path to the loctree binary
 * @throws Error if platform is unsupported or binary is not found
 */
export function getBinaryPath(): string;
