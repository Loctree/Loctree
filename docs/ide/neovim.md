# Neovim Setup

> **Part of [loctree-suite](https://github.com/Loctree/loctree-suite)**
> The LSP server and editor integrations ship with loctree-suite.
> Install the free CLI with `cargo install loctree`, then upgrade to suite for IDE features.

Configure Neovim to use `loctree-lsp` for dead code detection and navigation.

## Prerequisites

- Neovim 0.8+
- [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig)
- [loctree-suite](https://github.com/Loctree/loctree-suite) with `loctree-lsp` binary
- Loctree CLI installed: `cargo install loctree`

## Configuration

Add to your Neovim config (`init.lua` or `lua/plugins/lsp.lua`):

```lua
-- Add loctree to lspconfig
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Define loctree LSP if not already defined
if not configs.loctree then
  configs.loctree = {
    default_config = {
      cmd = { 'loctree-lsp' },
      filetypes = {
        'typescript', 'typescriptreact',
        'javascript', 'javascriptreact',
        'rust', 'python', 'go', 'vue', 'svelte'
      },
      root_dir = lspconfig.util.root_pattern('.loctree', '.git'),
      settings = {},
    },
  }
end

-- Setup with your preferred options
lspconfig.loctree.setup({
  on_attach = function(client, bufnr)
    -- Your on_attach function
    -- Loctree provides: diagnostics, hover, definition, references
  end,
})
```

## Lazy.nvim Example

```lua
{
  'neovim/nvim-lspconfig',
  config = function()
    local lspconfig = require('lspconfig')
    local configs = require('lspconfig.configs')

    if not configs.loctree then
      configs.loctree = {
        default_config = {
          cmd = { 'loctree-lsp' },
          filetypes = { 'typescript', 'javascript', 'rust', 'python' },
          root_dir = lspconfig.util.root_pattern('.loctree', '.git'),
        },
      }
    end

    lspconfig.loctree.setup({})
  end,
}
```

## Features

### Diagnostics

Dead exports, cycles, and twins appear as LSP diagnostics:

```
W: Export 'unusedFunction' has 0 imports [loctree:dead-export]
W: Circular import: a.ts → b.ts → a.ts [loctree:cycle]
I: Symbol 'Config' also exported from 3 files [loctree:twin]
```

### Hover

`:lua vim.lsp.buf.hover()` or `K` shows:

```
Export: useAuth
─────────────────
12 imports across 8 files
Top consumers: App.tsx, Login.tsx, Dashboard.tsx
```

### Go to Definition

`gd` jumps to the original export location, resolving re-export chains.

### References

`gr` lists all files importing the symbol.

## Keybindings

Suggested mappings (add to your config):

```lua
vim.keymap.set('n', 'gd', vim.lsp.buf.definition, { desc = 'Go to definition' })
vim.keymap.set('n', 'gr', vim.lsp.buf.references, { desc = 'Find references' })
vim.keymap.set('n', 'K', vim.lsp.buf.hover, { desc = 'Hover info' })
vim.keymap.set('n', '<leader>ca', vim.lsp.buf.code_action, { desc = 'Code actions' })
vim.keymap.set('n', '<leader>lr', ':!loct<CR>', { desc = 'Refresh loctree' })
```

## Troubleshooting

### LSP not starting

```vim
:LspInfo
```

Check if loctree is listed and running.

### No diagnostics

Ensure a snapshot exists (run `loct` once):

```bash
loct  # Generate snapshot
```

### Check LSP logs

```vim
:lua vim.cmd('edit ' .. vim.lsp.get_log_path())
```

---

*VibeCrafted with AI Agents (c)2024-2026 VetCoders*
