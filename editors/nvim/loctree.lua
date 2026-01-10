-- Loctree LSP configuration for Neovim
--
-- Add this to your Neovim config (e.g., ~/.config/nvim/lua/plugins/loctree.lua)
--
-- Created by M&K (c)2025 The LibraxisAI Team

-- Option 1: Using nvim-lspconfig (recommended)
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Register loctree as a custom LSP server
if not configs.loctree then
  configs.loctree = {
    default_config = {
      cmd = { 'loctree-lsp' },
      filetypes = { 'typescript', 'typescriptreact', 'javascript', 'javascriptreact', 'rust', 'python', 'go' },
      root_dir = function(fname)
        return lspconfig.util.root_pattern('.loctree', '.git')(fname)
      end,
      settings = {},
    },
  }
end

-- Setup with your preferred on_attach and capabilities
lspconfig.loctree.setup {
  on_attach = function(client, bufnr)
    -- Enable hover, go-to-definition, references
    local opts = { noremap = true, silent = true, buffer = bufnr }

    vim.keymap.set('n', 'K', vim.lsp.buf.hover, opts)
    vim.keymap.set('n', 'gd', vim.lsp.buf.definition, opts)
    vim.keymap.set('n', 'gr', vim.lsp.buf.references, opts)
    vim.keymap.set('n', '<leader>ca', vim.lsp.buf.code_action, opts)

    -- Loctree-specific commands
    vim.keymap.set('n', '<leader>lr', ':!loct<CR>', { desc = 'Loctree: Refresh' })
    vim.keymap.set('n', '<leader>lh', ':!loct health<CR>', { desc = 'Loctree: Health' })
    vim.keymap.set('n', '<leader>li', function()
      local file = vim.fn.expand('%:.')
      vim.cmd('!loct impact "' .. file .. '"')
    end, { desc = 'Loctree: Impact' })
  end,
  capabilities = vim.lsp.protocol.make_client_capabilities(),
}

-- Option 2: Manual LSP setup (if not using lspconfig)
--[[
vim.api.nvim_create_autocmd('FileType', {
  pattern = { 'typescript', 'typescriptreact', 'javascript', 'javascriptreact', 'rust' },
  callback = function()
    vim.lsp.start({
      name = 'loctree',
      cmd = { 'loctree-lsp' },
      root_dir = vim.fs.dirname(vim.fs.find({ '.loctree', '.git' }, { upward = true })[1]),
    })
  end,
})
]]

-- Diagnostic signs (optional customization)
vim.fn.sign_define('DiagnosticSignWarn', { text = '⚠', texthl = 'DiagnosticSignWarn' })
vim.fn.sign_define('DiagnosticSignInfo', { text = '●', texthl = 'DiagnosticSignInfo' })
vim.fn.sign_define('DiagnosticSignHint', { text = '◌', texthl = 'DiagnosticSignHint' })

-- Status line integration (for lualine or similar)
-- Shows loctree diagnostic count
local function loctree_status()
  local diagnostics = vim.diagnostic.get(0, { source = 'loctree' })
  if #diagnostics == 0 then
    return '🌳 healthy'
  end
  return '🌳 ' .. #diagnostics .. ' issues'
end

-- Add to your lualine config:
-- sections = { lualine_x = { loctree_status, 'encoding', 'fileformat' } }
