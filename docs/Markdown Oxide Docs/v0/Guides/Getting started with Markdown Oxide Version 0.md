# Markdown Oxide Getting Started Guide

If you want to get started using Markdown-Oxide, you are in the right place!

# Setup 

First, we must integrate Markdown-oxide with your text editor.

Click the link for your editor:

- [[#Neovim]]
- [[#VSCode]]
- [[#Zed]]
- [[#Helix]]

## Installation and Editor Config

### Neovim

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
        
        -- Ensure that dynamicRegistration is enabled! This allows the LS to take into account actions like the
        -- Create Unresolved File code action, resolving completions for unindexed code blocks, ...
        capabilities.workspace = {
            didChangeWatchedFiles = {
              dynamicRegistration = true,
            },
        }
        
        require("lspconfig").markdown_oxide.setup({
            capabilities = capabilities, -- again, ensure that capabilities.workspace.didChangeWatchedFiles.dynamicRegistration = true
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

        Modify your lsp `on_attach` function to support opening daily notes with, for example, `:Daily two days ago` or `:Daily next monday`. The specifications can be found [here](<Daily Notes#Opening Daily Notes>)

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


### VSCode

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

### Zed

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

    

### Helix

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

## Configuration

![[v0 Configuration Reference#^configurationinfo]]

You can probably leave the settings as their defaults for now, but here are a few you may want to set.

- [Daily Note Format](<v0 Configuration Reference#Daily Note Format Config Option>). If you have obsidian, this one is automatically imported if not set. 
- Do you *not* use the first heading of your markdown files as the file title? You probably want to disable [[v0 Configuration Reference#Title Headings|title headings]].
- Do you have a specific folder where your daily notes are? Try [[v0 Configuration Reference#Daily Notes Folder]]. *This may be imported from Obsidian*
- Do you have a specific folder where new files should be created? Try [[v0 Configuration Reference#New Files Folder]]. *This may be imported from Obsidian*
- Do you want `.md` parsed and appended in your links? [[v0 Configuration Reference#MD Extension]]

# Using markdown-oxide

Now that you are set up, it is time to start using your new PKM! 

Most editors have an LSP interface that makes using Markdown-oxide's features intuitive. Over time, you will figure out a workflow that fits your needs. 

Additionally, you can reference the gifs and descriptions in the [[v0 Features Reference]] to figure out how to use different features. 

I am working toward a future where all of markdown-oxide's features are thoroughly documented, but we are not there yet!

# Future steps

As you use Markdown Oxide more, there may be behavior you want to configure. For this, check the [[v0 Configuration Reference]].

And if you want to have local access to the settings, here is the [default config file](<v0 Configuration Reference#Default Config File>) for you to copy and paste. 

I hope you enjoy! Feel free to open issues for bugs and features on the GitHub repo: https://github.com/Feel-ix-343/markdown-oxide



[^1]: ![[Documentation Notes#^docEmbeds]]
