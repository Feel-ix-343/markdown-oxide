
**markdown-oxide**: Robust, Minimalist, Unbundled PKM for your favorite text-editor through the LSP.

**[Quick Start](#quick-start)**

# Docs


Here are some recommended links from our documentation website, <https://oxide.md>

## Recommended Links

* [What is markdown-oxide?](https://oxide.md/v0/Articles/Markdown-Oxide+v0): An overview of our PKM features to help you determine if markdown-oxide is for you
* [Markdown-oxide getting-started guide](https://oxide.md/v0/Guides/Getting+started+with+Markdown+Oxide+Version+0): A guide to setting up your text editor, configuring the PKM, and using the features. 
* [Features Reference](https://oxide.md/v0/references/v0+Features+Reference): An organized list of all features
* [Configuration Reference](https://oxide.md/v0/references/v0+Configuration+Reference): Configuration information to reference
    + [Default Config File](https://oxide.md/v0/References/v0+Configuration+Reference#Default+Config+File)

# Quick Start

Get started with Markdown-oxide as fast as possible! 

Set up the PKM for your text editor...

- [Neovim](#Neovim)
- [VSCode](#VSCode)
- [Zed](#Zed)
- [Helix](#Helix)

## Neovim

1. Give Neovim access to the binary.

    - <details>
         <summary>Cargo Install (from source)</summary>
    
        ```bash
        cargo install --locked --git https://github.com/Feel-ix-343/markdown-oxide.git markdown-oxide
        ```
    
    </details>

    - <details>
         <summary>Cargo binstall (from hosted binary)</summary>
    
        ```bash
        cargo binstall --git 'https://github.com/feel-ix-343/markdown-oxide' markdown-oxide
        ```
    
    </details>
    
    - <details>
         <summary>AUR (from source)</summary>
    
        ```bash
        paru -S markdown-oxide-git
        ```

        ```bash
        yay -S markdown-oxide-git
        ```
    
    </details>

    - [Mason.nvim](https://github.com/williamboman/mason.nvim) (from hosted binary)
    - Nix Unstable: `pkgs.markdown-oxide`
  
2. Modify your Neovim Configuration ^nvimconfigsetup
    - <details>
        <summary>Modify LSP Config (making sure to adjust capabilities as follows)</summary>

        ```lua        
        -- An example nvim-lspconfig capabilities setting
        local capabilities = require("cmp_nvim_lsp").default_capabilities(vim.lsp.protocol.make_client_capabilities())
        
        require("lspconfig").markdown_oxide.setup({
            -- Ensure that dynamicRegistration is enabled! This allows the LS to take into account actions like the
            -- Create Unresolved File code action, resolving completions for unindexed code blocks, ...
            capabilities = vim.tbl_deep_extend(
                'force',
                capabilities,
                {
                    workspace = {
                        didChangeWatchedFiles = {
                            dynamicRegistration = true,
                        },
                    },
                }
            ),
            on_attach = on_attach -- configure your on attach config
        })
        ```

    </details> 

    - <details>
        <summary>Modify your nvim-cmp configuration</summary>

        Modify your nvim-cmp source settings for nvim-lsp (note: you must have nvim-lsp installed)

        ```lua        
        {
        name = 'nvim_lsp',
          option = {
            markdown_oxide = {
              keyword_pattern = [[\(\k\| \|\/\|#\)\+]]
            }
          }
        },
        ```

    </details>

    - <details>
        <summary>(optional) Enable Code Lens (eg for UI reference count)</summary>

        Modify your lsp `on_attach` function.

        ```lua
        local function check_codelens_support()
        local clients = vim.lsp.get_active_clients({ bufnr = 0 })
        for _, c in ipairs(clients) do
          if c.server_capabilities.codeLensProvider then
            return true
          end
        end
        return false
        end

        vim.api.nvim_create_autocmd({ 'TextChanged', 'InsertLeave', 'CursorHold', 'LspAttach', 'BufEnter' }, {
        buffer = bufnr,
        callback = function ()
          if check_codelens_support() then
            vim.lsp.codelens.refresh({bufnr = 0})
          end
        end
        })
        -- trigger codelens refresh
        vim.api.nvim_exec_autocmds('User', { pattern = 'LspAttached' })
        ```

    </details>

    - <details>
        <summary>(optional) Enable opening daily notes with natural langauge</summary>

        Modify your lsp `on_attach` function to support opening daily notes with, for example, `:Daily two days ago` or `:Daily next monday`. 

        ```lua
        -- setup Markdown Oxide daily note commands
        if client.name == "markdown_oxide" then

          vim.api.nvim_create_user_command(
            "Daily",
            function(args)
              local input = args.args

              vim.lsp.buf.execute_command({command="jump", arguments={input}})

            end,
            {desc = 'Open daily note', nargs = "*"}
          )
        end
        ```

    </details>    
- Ensure relevant plugins are installed:
    * [Nvim CMP](https://github.com/hrsh7th/nvim-cmp): UI for using LSP completions
    * [Telescope](https://github.com/nvim-telescope/telescope.nvim): UI helpful for the LSP references implementation
        - Allows you to view and fuzzy match backlinks to files, headings, and blocks.
    * [Lspsaga](https://github.com/nvimdev/lspsaga.nvim): UI generally helpful for LSP commands
        + Allows you to edit linked markdown files in a popup window, for example. 


## VSCode

Install the [vscode extension](https://marketplace.visualstudio.com/items?itemName=FelixZeller.markdown-oxide) (called `Markdown Oxide`). As for how the extension uses the language server, there are two options
1. Recommended: the extension will download the server's binary and use that
2. The extension will use `markdown-oxide` from path. To install to your path, there are the following methods for VSCode:

    - <details>
         <summary>Cargo Install (from source)</summary>
    
        ```bash
        cargo install --locked --git https://github.com/Feel-ix-343/markdown-oxide.git markdown-oxide
        ```
    
    </details>

    - <details>
         <summary>Cargo binstall[1] (from hosted binary)</summary>
    
        ```bash
        cargo binstall --git 'https://github.com/feel-ix-343/markdown-oxide' markdown-oxide
        ```
    
    </details>
    
    - <details>
         <summary>AUR (from source)</summary>
    
        ```bash
        paru -S markdown-oxide-git
        ```

        ```bash
        yay -S markdown-oxide-git
        ```
    
    </details>
    
    - Nix Unstable: `pkgs.markdown-oxide`

## Zed

Markdown Oxide is available as an extension titled `Markdown Oxide`. Similarly to VSCode, there are two methods for this extension to access the language server
1. Recommended: the extension will download the server's binary and use that
2. The extension will use `markdown-oxide` from path. To install to your path, there are the following methods for Zed:

    - <details>
         <summary>Cargo Install (from source)</summary>
    
        ```bash
        cargo install --locked --git https://github.com/Feel-ix-343/markdown-oxide.git markdown-oxide
        ```
    
    </details>

    - <details>
         <summary>Cargo binstall[1] (from hosted binary)</summary>
    
        ```bash
        cargo binstall --git 'https://github.com/feel-ix-343/markdown-oxide' markdown-oxide
        ```
    
    </details>
    
    - <details>
         <summary>AUR (from source)</summary>
    
        ```bash
        paru -S markdown-oxide-git
        ```

        ```bash
        yay -S markdown-oxide-git
        ```
    
    </details>
    
    - Nix Unstable: `pkgs.markdown-oxide`

    

## Helix

For Helix, all you must do is install the language server's binary to your path. The following installation methods are available:
- <details>
     <summary>Cargo Install (from source)</summary>

    ```bash
    cargo install --locked --git https://github.com/Feel-ix-343/markdown-oxide.git markdown-oxide
    ```

</details>

- <details>
    <summary>Cargo binstall[1] (from hosted binary)</summary>
    
    ```bash
    cargo binstall --git 'https://github.com/feel-ix-343/markdown-oxide' markdown-oxide
    ```
    
</details>

- <details>
     <summary>AUR (from source)</summary>

    ```bash
    paru -S markdown-oxide-git
    ```

    ```bash
    yay -S markdown-oxide-git
    ```

</details>

- Nix Unstable: `pkgs.markdown-oxide`
