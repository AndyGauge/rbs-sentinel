# Steep-only, no in-editor lint

If you'd rather not register a second language server at all, keep `sentinel watch` running as a plain background process and let Steep be the only LSP attached to your Ruby files. You still get automatic `.rbs` regeneration — and therefore live Steep diagnostics — you just lose Sentinel's own lint diagnostics in the editor (they print to whichever terminal is running `watch` instead).

## Neovim: launch it from Steep's `on_attach`

```lua
-- Add this to your lsp.lua, outside the return table or inside on_attach
local function start_sentinel()
  if _G.sentinel_job_id then return end -- Don't start it twice

  _G.sentinel_job_id = vim.fn.jobstart({ "bundle", "exec", "sentinel", "watch" }, {
    detach = true, -- Process keeps running in background
    on_stderr = function(_, data)
      -- Optional: print errors to Neovim's :messages if something breaks
      if data and data[1] ~= "" then
        vim.schedule(function()
          vim.notify("Sentinel Error: " .. table.concat(data, "\n"), vim.log.levels.ERROR)
        end)
      end
    end,
    on_exit = function()
      _G.sentinel_job_id = nil
    end,
  })
end

-- Update your steep setup to trigger this
require('lspconfig').steep.setup({
  on_attach = function(client, bufnr)
    start_sentinel()
    -- ... rest of your on_attach
  end,
  -- ... rest of your config
})
```

## VS Code / Zed: a terminal or task

There's no `on_attach` hook to piggyback on outside Neovim, so the straightforward version is just running it yourself:

```bash
bundle exec sentinel watch
```

...in a terminal tab you leave open, or wired up as a VS Code task / Zed task that starts on folder open. Either way, the moment you save a `.rb` file, `sig/generated` refreshes and your editor's Steep client picks it up on its own next check.

## Why you'd pick this over `sentinel lsp`

- You don't want a second LSP client's diagnostics interleaved with Steep's in the same panel.
- You're on an editor without straightforward multi-server-per-language support.
- You just haven't set up [the LSP option](../editor-feedback.md) yet and this was already working.

There's no correctness difference in the generated `.rbs` — `sentinel lsp` and `sentinel watch` share the same transpile-and-write code path. The only thing you lose is Sentinel's own lint diagnostics showing up in-editor instead of in a terminal.
