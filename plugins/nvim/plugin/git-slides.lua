-- git-slides.lua - drive a git-slides presentation from Neovim.
-- Part of git-slides <https://github.com/qrichert/git-slides>.
-- License: MIT OR Apache-2.0. Requires Neovim 0.10+.
--
-- Note: To be fair, I didn't write a single line of this. I handed it
-- to Claude (Opus 4.8) and had it extensively reviewed and fixed by
-- Codex (Sol 5.5) to make it "bulletproof". I use Neovim myself, but I
-- have never used Lua beyond basic configuration, let alone written a
-- plugin. Just to say it should be quite good, but I have no way to
-- truly assess quality. I can only trust the dozens of back-and-forths
-- until neither Claude nor Codex could find any issue (as of 2026-08).
-- First I tried making the plugin work with both Vim and Neovim. Bad
-- idea. It was an endless stream of pain. The Neovim plugin alone was
-- tedious to get right, with its own fair share of edge cases.
--
-- Edit: Actually no, the "until neither Claude nor Codex could find any
-- issue" was the initial plan, but after the dozen iterations it keeps
-- on finding new edge cases, and I'm just burning tokens for probably
-- not much added value. Maybe worth a periodic check-and-fix, but I'm
-- stopping here for now, it's good enough.
--
-- See `:help git-slides` (doc/git-slides.txt) for install, commands,
-- mappings, and config.
--
-- Every command runs the `git-slides` binary directly via `vim.system`
-- (no shell, so no escaping and no POSIX-only restriction); navigation
-- checks out a commit, so repository file buffers are reloaded, or
-- replaced by a placeholder when the checkout removed the file.

if vim.g.loaded_git_slides then
  return
end
vim.g.loaded_git_slides = true

-- Path separators differ by OS: Windows' resolve() yields backslashes
-- and treats them as separators, while on POSIX '\' is a legal filename
-- character. Gate every separator-aware branch (exe, normsep) on this.
local is_windows = vim.fn.has('win32') == 1

-- Vimscript sets flags as 0/1, but Lua counts 0 as truthy, so a
-- `let g:...= 0` would read as "on". Normalize: nil/false/0 are off.
local function truthy(v)
  return v ~= nil and v ~= false and v ~= 0
end

-- Forward declarations so callers sit above callees (stepdown order);
-- Lua needs the locals declared before the functions that reference
-- them. `setup` is the entry point, invoked on load at the bottom.
local setup
local register_commands, register_plug_maps, register_default_maps
local cb_start, cb_stop, cb_next, cb_previous, cb_go, cb_status, cb_list
local slides, start
local run
local cwd, exe, repo_root
local bufs_under, recurses_submodules, in_repo, submodule_of
local placeholders_under, root_prefix, normfile, normdir, fold, normsep
local reload_targets, fingerprints, changed, synced, fingerprint
local reload, revert, revive, drop
local echo_error, echo_warn, echo_output

-- --- Setup ---

-- Register commands, <Plug> maps, and (unless disabled) the default
-- maps. Reads load-time config; `git_slides_executable` is read later,
-- at call time.
function setup()
  register_commands()
  register_plug_maps()
  if not truthy(vim.g.git_slides_no_default_mappings) then
    register_default_maps()
  end
end

-- --- Commands ---

-- `count` lets `:GitSlidesGo 3`, `:3GitSlidesGo`, and count-maps share
-- one path (no count is 0); `bang` forces past the unsaved-buffer
-- guard.
function register_commands()
  local command = vim.api.nvim_create_user_command
  command('GitSlidesStart', function(o) start(o.args, o.bang) end, { nargs = '?', bang = true })
  command('GitSlidesStop', function(o) slides('stop', 0, o.bang) end, { bang = true })
  command('GitSlidesNext', function(o) slides('next', o.count, o.bang) end, { count = 1, bang = true })
  command('GitSlidesPrevious', function(o) slides('previous', o.count, o.bang) end, { count = 1, bang = true })
  command('GitSlidesGo', function(o) slides('go', o.count, o.bang) end, { count = 0, bang = true })
  command('GitSlidesStatus', function() slides('status', 0, false) end, {})
  command('GitSlidesList', function() slides('list', 0, false) end, {})
end

-- --- Mappings ---

-- Remappable entry points; the counted ones read v:count (0 when none
-- typed).
function register_plug_maps()
  local map = vim.keymap.set
  map('n', '<Plug>(GitSlidesStart)', cb_start, { silent = true })
  map('n', '<Plug>(GitSlidesStop)', cb_stop, { silent = true })
  map('n', '<Plug>(GitSlidesNext)', cb_next, { silent = true })
  map('n', '<Plug>(GitSlidesPrevious)', cb_previous, { silent = true })
  map('n', '<Plug>(GitSlidesGo)', cb_go, { silent = true })
  map('n', '<Plug>(GitSlidesStatus)', cb_status, { silent = true })
  map('n', '<Plug>(GitSlidesList)', cb_list, { silent = true })
end

-- Default <Leader>s maps bind straight to the callbacks (not the <Plug>
-- strings, which would need remap=true to fire). `unique` + pcall means
-- an already-taken LHS is left untouched: the user's mapping always
-- wins.
function register_default_maps()
  local prefix = vim.g.git_slides_mapping_prefix or '<Leader>s'
  local defaults = {
    { 's', cb_start },
    { 'S', cb_stop },
    { 'n', cb_next },
    { 'N', cb_previous },
    { 'p', cb_previous },
    { 'g', cb_go },
  }
  for _, m in ipairs(defaults) do
    pcall(vim.keymap.set, 'n', prefix .. m[1], m[2], { unique = true, silent = true })
  end
end

-- Shared by the <Plug> maps and the default maps.
function cb_start() start('', false) end
function cb_stop() slides('stop', 0, false) end
function cb_next() slides('next', vim.v.count, false) end
function cb_previous() slides('previous', vim.v.count, false) end
function cb_go() slides('go', vim.v.count, false) end
function cb_status() slides('status', 0, false) end
function cb_list() slides('list', 0, false) end

-- --- Argument builders ---

-- Counts are stringified here so argv holds only strings.
function slides(subcmd, count, bang)
  if subcmd == 'go' then
    if count < 1 then
      echo_error('git-slides: go needs a slide number (e.g. 3<Leader>sg)')
      return
    end
    return run({ 'go', tostring(count) }, true, bang)
  elseif subcmd == 'next' or subcmd == 'previous' then
    local argv = count > 0 and { subcmd, tostring(count) } or { subcmd }
    return run(argv, true, bang)
  elseif subcmd == 'stop' then
    return run({ 'stop' }, true, bang)
  else
    -- status / list, non-mutating: no guard, no reload.
    return run({ subcmd }, false, bang)
  end
end

function start(ref, bang)
  local argv = ref == '' and { 'start' } or { 'start', ref }
  return run(argv, true, bang)
end

-- --- Core ---

-- The single git-slides invocation point. `argv` is a list of strings.
-- `mutating` gates the unsaved-buffer guard and the post-run reload;
-- `bang` bypasses the guard.
function run(argv, mutating, bang)
  local dir = cwd()
  -- Computed for every mutating call so the reload runs on the bang
  -- path too.
  local root = mutating and repo_root(dir) or nil

  local bufs = {}
  local before = {}
  local phs = {}
  if mutating and root then
    bufs = bufs_under(root)
    before = fingerprints(bufs)
    phs = placeholders_under(root)
    if not bang then
      local unsaved = 0
      for _, b in ipairs(bufs) do
        if vim.bo[b].modified then unsaved = unsaved + 1 end
      end
      if unsaved > 0 then
        echo_error(string.format(
          'git-slides: %d unsaved buffer(s) in this repo; :w or :e! first, or use :GitSlides...!',
          unsaved))
        return
      end
    end
  end

  local run_dir = (mutating and root) or dir
  local run_exe = exe()
  local cmd = { run_exe }
  vim.list_extend(cmd, argv)
  local ok, res = pcall(function()
    return vim.system(cmd, { cwd = run_dir, text = true }):wait()
  end)

  if not ok then
    echo_error('git-slides: could not run ' .. run_exe .. ': ' .. tostring(res))
    return
  end

  -- Reload the buffers whose file changed on disk (content differs, or
  -- the file appeared/disappeared), success or failure: git-slides can
  -- mutate the worktree and then exit nonzero (stash-then-failed-checkout;
  -- checkout-then-failed-store-removal), so comparing bytes still catches
  -- those while sparing every unrelated buffer's cursor and undo. On
  -- success also reload any still-modified buffer: a bang forced past the
  -- guard promising to discard its edits. On a spawn failure (handled
  -- above) nothing ran.
  -- vim.system() reports a signal-killed child as { code = 0, signal =
  -- 15 } on POSIX, so success needs both a zero code and no signal.
  local succeeded = res.code == 0 and (res.signal or 0) == 0
  if mutating and root then
    reload(root, reload_targets(before, bufs, succeeded))
    revive(phs)
  end

  if not succeeded then
    -- Concatenating stdout then stderr reproduces the old `2>&1`
    -- order: git-slides emits its progress on stdout before the
    -- terminal error.
    local combined = ((res.stdout or '') .. (res.stderr or '')):gsub('\n+$', '')
    local lines = {}
    for _, line in ipairs(vim.split(combined, '\n', { plain = true, trimempty = true })) do
      table.insert(lines, 'git-slides: ' .. line)
    end
    if #lines == 0 then
      -- A signal kill (or a bare nonzero exit) can leave no output; still
      -- report, never fail silently.
      lines = {
        (res.signal or 0) ~= 0
          and ('git-slides: terminated by signal ' .. res.signal)
          or ('git-slides: exited with code ' .. tostring(res.code)),
      }
    end
    echo_error(table.concat(lines, '\n'))
  else
    local subcmd = argv[1]
    if subcmd == 'start' then
      echo_output('git-slides: presentation started')
    elseif subcmd == 'stop' then
      echo_output('git-slides: presentation ended')
    elseif subcmd == 'status' or subcmd == 'list' then
      local out = (res.stdout or ''):gsub('\n+$', '')
      if out ~= '' then echo_output(out) end
    end
    local err = (res.stderr or ''):gsub('\n+$', '')
    if err ~= '' then
      local lines = {}
      for _, line in ipairs(vim.split(err, '\n', { plain = true, trimempty = true })) do
        table.insert(lines, 'git-slides: ' .. line)
      end
      echo_warn(table.concat(lines, '\n'))
    end
  end
end

-- Repository context retained after the displayed buffer was dropped, or
-- Neovim's working directory. Basing the fallback on cwd (not the
-- focused buffer's own dir) means a GitSlides command targets the repo
-- the user is cd'd into, even when a :help, terminal, or out-of-repo
-- buffer is focused. Walking up keeps the next `cwd` (even `stop`) from
-- running against a vanished path.
function cwd()
  local root = vim.b.git_slides_root
  if root and root ~= '' and vim.fn.isdirectory(root) == 1 then
    return root
  end

  local dir = vim.fn.getcwd()
  while dir ~= '' and vim.fn.isdirectory(dir) == 0 do
    local parent = vim.fn.fnamemodify(dir, ':h')
    if parent == dir then break end
    dir = parent
  end
  return vim.fn.isdirectory(dir) == 1 and dir or vim.fn.getcwd()
end

-- The git-slides executable. A relative path is absolutized so the run
-- directory can't strand it; a bare name is left for PATH resolution. On
-- Windows '\' is a separator too, so a backslash path is treated as
-- relative (on POSIX '\' is a filename character, so it isn't).
function exe()
  local e = vim.g.git_slides_executable or 'git-slides'
  if e:find('/', 1, true) or (is_windows and e:find('\\', 1, true)) then
    e = vim.fn.fnamemodify(e, ':p')
  end
  return e
end

-- Repository root for `dir` (git's physical --show-toplevel), or nil
-- when not in a repository. Running the binary directly means an
-- embedded newline in the path survives; only the single trailing
-- newline is stripped.
function repo_root(dir)
  -- vim.system raises if git is not on PATH (unlike the old system());
  -- pcall so a missing git yields "not in a repo", not a stack trace.
  local ok, res = pcall(function()
    return vim.system({ 'git', '-C', dir, 'rev-parse', '--show-toplevel' }, { text = true }):wait()
  end)
  if not ok or res.code ~= 0 then return nil end
  local out = (res.stdout or ''):gsub('\n$', '')
  -- Empty is not a valid root; return nil so callers skip the
  -- guard/reload (an empty string is truthy in Lua, unlike Vimscript's
  -- empty()).
  if out == '' then return nil end
  return out
end

-- --- Path membership ---
--
-- A buffer belongs to the repo when its slot sits under the repo root.
-- Raw-string comparison is wrong on real filesystems three ways:
--   1. Symlinks in the DIRECTORY prefix (macOS /tmp -> /private/tmp)
--      spell the same place differently, so resolve the directory.
--   2. A tracked file that is ITSELF a symlink pointing outside the
--      repo still belongs to it (membership is by directory entry, not
--      target), so resolve only the parent dir and keep the final
--      component.
--   3. Case-insensitive filesystems (macOS/Windows, per
--      'fileignorecase') spell the same path in different case, so fold
--      case when they do.

-- Loaded, file-backed buffers whose slot lives under `root`. An empty
-- root matches nothing (guards against '' turning into the '/' prefix).
function bufs_under(root)
  if not root or root == '' then return {} end
  local prefix = root_prefix(root)
  local recurse = recurses_submodules(root)
  local memberships = {}
  local bufs = {}
  for _, b in ipairs(vim.api.nvim_list_bufs()) do
    local name = vim.api.nvim_buf_get_name(b)
    if vim.api.nvim_buf_is_loaded(b)
      and vim.bo[b].buftype == ''
      and name ~= ''
      and vim.fn.isdirectory(name) == 0
      and normfile(name):find(prefix, 1, true) == 1
      and in_repo(name, root, recurse, memberships)
    then
      table.insert(bufs, b)
    end
  end
  return bufs
end

-- Whether an outer checkout will recurse into active submodules. Read once
-- per scan; unset, invalid, and failed config lookups all mean false, matching
-- Git's default checkout behavior.
function recurses_submodules(root)
  local ok, res = pcall(function()
    return vim.system(
      { 'git', '-C', root, 'config', '--bool', 'submodule.recurse' },
      { text = true }
    ):wait()
  end)
  return ok and res.code == 0 and vim.trim(res.stdout or '') == 'true'
end

-- A file lexically under `root` may sit in a closer repository. A `.git`
-- directory is an independent worktree and stays excluded. A `.git` file can
-- be either a submodule or a linked worktree; include it only when recursive
-- checkout is enabled and Git identifies its superproject chain as belonging
-- to `root`. Cache that query for other buffers in the same nested worktree.
function in_repo(name, root, recurse, memberships)
  local target = normdir(root)
  local dir = vim.fn.fnamemodify(name, ':p:h')
  while dir ~= '' do
    local marker = dir .. '/.git'
    local directory = vim.fn.isdirectory(marker) == 1
    local file = vim.fn.filereadable(marker) == 1
    if directory or file then
      if normdir(dir) == target then return true end
      if directory or not recurse then return false end
      if memberships[dir] == nil then
        memberships[dir] = submodule_of(dir, root)
      end
      return memberships[dir]
    end
    local parent = vim.fn.fnamemodify(dir, ':h')
    if parent == dir then break end
    dir = parent
  end
  return false
end

-- Whether `dir` is a submodule (possibly nested) of `root`. Linked worktrees
-- have a `.git` file too, but report no superproject and therefore stay out.
function submodule_of(dir, root)
  local target = normdir(root)
  local current = dir
  while current ~= '' do
    local ok, res = pcall(function()
      return vim.system(
        { 'git', '-C', current, 'rev-parse', '--show-superproject-working-tree' },
        { text = true }
      ):wait()
    end)
    if not ok or res.code ~= 0 then return false end
    local super = (res.stdout or ''):gsub('\n$', '')
    if super == '' then return false end
    if normdir(super) == target then return true end
    current = super
  end
  return false
end

-- Windows showing a git-slides placeholder whose original file sat under
-- `root`, captured before a mutating command so a checkout that restores
-- the file can reopen it (see revive()).
function placeholders_under(root)
  if not root or root == '' then return {} end
  local prefix = root_prefix(root)
  local out = {}
  for _, win in ipairs(vim.api.nvim_list_wins()) do
    local buf = vim.api.nvim_win_get_buf(win)
    local ok, path = pcall(vim.api.nvim_buf_get_var, buf, 'git_slides_file')
    if ok and type(path) == 'string' and path ~= ''
      and normfile(path):find(prefix, 1, true) == 1
    then
      table.insert(out, { win = win, buf = buf, path = path })
    end
  end
  return out
end

-- The normalized '<root>/' prefix a member path must start with, with
-- exactly one trailing separator. Stripping any trailing separator first
-- keeps a volume root ('C:/') or POSIX root ('/') from becoming 'C://'
-- or '//', which real member paths never match.
function root_prefix(root)
  return (normdir(root):gsub('/+$', '')) .. '/'
end

-- Resolve a file's PARENT dir but keep its final component, then
-- case-fold.
function normfile(path)
  local full = vim.fn.fnamemodify(path, ':p')
  return fold(vim.fn.resolve(vim.fn.fnamemodify(full, ':h')) .. '/' .. vim.fn.fnamemodify(full, ':t'))
end

-- Fully resolve a directory (prefix symlinks), then case-fold.
function normdir(dir)
  return fold(vim.fn.resolve(dir))
end

-- Case-fold (when the filesystem ignores case) and normalize path
-- separators so two spellings of the same path compare equal.
function fold(path)
  path = normsep(path)
  return vim.o.fileignorecase and path:lower() or path
end

-- On Windows, resolve() yields backslashes; rewrite them to '/' so the
-- prefix test in bufs_under() holds for files in subdirectories. A
-- no-op on POSIX, where '\' is a valid filename character and must stay
-- intact.
function normsep(path)
  return is_windows and (path:gsub('\\', '/')) or path
end

-- --- Change detection ---
--
-- Reload targets the buffers a mutating command actually changed, found
-- by fingerprinting each captured file before and after: content
-- differs, or it appeared/disappeared. This spares unrelated buffers'
-- cursor and undo, and makes a command that mutated nothing (e.g. an
-- errored start, even with a bang) reload nothing. Byte-exact, so unlike
-- mtime/size/autoread it never misses a same-size, same-time checkout.

-- Buffers to reload after a mutation: those whose file changed on disk, any
-- unmodified buffer already out of sync with disk, plus (only when the command
-- succeeded) any buffer left modified. The guard already blocks unsaved
-- buffers, so a modified one here means a bang forced past it promising to
-- discard the edits; honor that once navigation actually happened.
-- Deduplicated via a set.
function reload_targets(before, bufs, succeeded)
  local set = {}
  for _, b in ipairs(changed(before)) do set[b] = true end
  for _, b in ipairs(bufs) do
    if vim.api.nvim_buf_is_valid(b) then
      if not vim.bo[b].modified and not synced(b) then
        set[b] = true
      elseif succeeded and vim.bo[b].modified then
        set[b] = true
      end
    end
  end
  local out = {}
  for b in pairs(set) do table.insert(out, b) end
  return out
end

-- Fingerprint every captured buffer's file, keyed by buffer number.
function fingerprints(bufs)
  local fps = {}
  for _, b in ipairs(bufs) do
    fps[b] = fingerprint(vim.api.nvim_buf_get_name(b))
  end
  return fps
end

-- The captured buffers whose file changed since `before` (content
-- differs, or it appeared/disappeared).
function changed(before)
  local bufs = {}
  for buf, fp in pairs(before) do
    if vim.api.nvim_buf_is_valid(buf)
      and fingerprint(vim.api.nvim_buf_get_name(buf)) ~= fp
    then
      table.insert(bufs, buf)
    end
  end
  return bufs
end

-- Whether the loaded buffer text matches its file. Decode the raw bytes using
-- the buffer's detected file encoding, then mirror Neovim's BOM, line-ending,
-- and final-EOL normalization before comparing lines. Failures are treated as
-- out of sync so navigation repairs rather than silently preserves stale text.
function synced(buf)
  local disk = fingerprint(vim.api.nvim_buf_get_name(buf))
  if disk == false then return false end

  local fenc = vim.bo[buf].fileencoding
  if fenc ~= '' and fenc ~= vim.o.encoding then
    local ok, decoded = pcall(vim.iconv, disk, fenc, vim.o.encoding)
    if not ok or decoded == nil then return false end
    disk = decoded
  end
  if vim.bo[buf].bomb and disk:sub(1, 3) == '\239\187\191' then
    disk = disk:sub(4)
  end

  local separator = ({ dos = '\r\n', mac = '\r' })[vim.bo[buf].fileformat] or '\n'
  local has_eol = disk == '' or disk:sub(-#separator) == separator
  if vim.bo[buf].endofline ~= has_eol then return false end

  local lines = vim.split(disk, separator, { plain = true })
  if #lines == 0 then lines = { '' } end
  if has_eol and #lines > 1 then table.remove(lines) end
  return vim.deep_equal(vim.api.nvim_buf_get_lines(buf, 0, -1, true), lines)
end

-- A file's raw bytes, or false when absent. Comparing bytes directly
-- (rather than a vim.fn.sha256 digest, which rejects the NUL bytes in
-- binary content with E976) keeps a NUL/LF swap a real change. false
-- (not nil) so it survives as a table value and an appeared/disappeared
-- file still counts.
function fingerprint(name)
  local fd = io.open(name, 'rb')
  if not fd then return false end
  local content = fd:read('*a')
  fd:close()
  return content or ''
end

-- --- Reload / drop ---

-- Explicitly re-read or drop every file buffer captured before a
-- mutating command. This does not depend on timestamps, sizes, modes,
-- or 'autoread'.
function reload(root, bufs)
  for _, b in ipairs(bufs) do
    revert(b, root)
  end
end

-- Re-read one captured buffer after a mutating command even when its
-- metadata did not change; drop it only when the file is actually gone.
-- `nvim_buf_call` reloads the buffer without touching the current
-- window, so, unlike the borrow-a-window trick, no bufhidden
-- neutralization is needed.
function revert(buf, root)
  if not vim.api.nvim_buf_is_valid(buf) then return end

  local name = vim.api.nvim_buf_get_name(buf)
  if vim.fn.filereadable(name) == 0 then
    drop(buf, root)
    return
  end

  local ok, err = pcall(vim.api.nvim_buf_call, buf, function()
    vim.cmd('silent keepalt keepjumps edit!')
  end)
  if not ok then
    -- Two ways the reload fails: the file vanished mid-read (E211 race)
    -- -> drop it; or a user autocmd (BufReadPost) erred while the file
    -- is fine -> keep the still-readable buffer and report, never delete
    -- valid content over someone else's hook.
    if vim.fn.filereadable(name) == 0 then
      drop(buf, root)
    else
      echo_error('git-slides: could not reload ' .. name .. ': ' .. tostring(err))
    end
  end
end

-- Checkout removed the file. Install a fresh placeholder in every
-- window showing the buffer FIRST, then wipe it. A per-window buffer is
-- essential: one shared bufhidden=wipe placeholder would self-destruct
-- on the first window's hide and blank the others. winfixbuf blocks the
-- swap, so clear and restore it around each. b:git_slides_root lets
-- cwd() still resolve the repo; b:git_slides_file records the path so
-- revive() can reopen the file if a later slide restores it.
function drop(buf, root)
  if not vim.api.nvim_buf_is_valid(buf) then return end
  local name = vim.api.nvim_buf_get_name(buf)

  for _, win in ipairs(vim.fn.win_findbuf(buf)) do
    local scratch = vim.api.nvim_create_buf(false, true)
    local fixed = vim.wo[win].winfixbuf
    if fixed then vim.wo[win].winfixbuf = false end
    vim.api.nvim_win_set_buf(win, scratch)
    vim.bo[scratch].bufhidden = 'wipe'
    vim.bo[scratch].swapfile = false
    -- Nonmodifiable: a nofile buffer never sets 'modified', so text typed
    -- into it is invisible to the unsaved guard and bufhidden=wipe would
    -- silently discard it when the file returns.
    vim.bo[scratch].modifiable = false
    vim.api.nvim_buf_set_var(scratch, 'git_slides_root', root or '')
    vim.api.nvim_buf_set_var(scratch, 'git_slides_file', name)
    if fixed then vim.wo[win].winfixbuf = true end
  end

  if vim.api.nvim_buf_is_valid(buf) then
    vim.api.nvim_buf_delete(buf, { force = true })
  end
  echo_warn('git-slides: file no longer available: ' .. name)
end

-- Reopen the real file in each captured placeholder window whose path is
-- readable again (a later slide re-added it), mirroring drop()'s
-- winfixbuf dance. bufadd/bufload (not `:edit <path>`) tolerates any
-- path, e.g. an embedded newline. Skip a window the swap-out already
-- moved off the placeholder.
function revive(phs)
  for _, ph in ipairs(phs) do
    if vim.api.nvim_win_is_valid(ph.win)
      and vim.api.nvim_win_get_buf(ph.win) == ph.buf
      and vim.fn.filereadable(ph.path) == 1
    then
      local newbuf = vim.fn.bufadd(ph.path)
      local ok, err = pcall(vim.fn.bufload, newbuf)
      if not ok then
        echo_error('git-slides: could not reload ' .. ph.path .. ': ' .. tostring(err))
      end
      -- BufReadPost runs after the file is loaded, so a failing hook still
      -- leaves valid content that can replace the placeholder. Earlier load
      -- failures leave it in place for a later navigation to retry.
      if vim.api.nvim_buf_is_loaded(newbuf) then
        local fixed = vim.wo[ph.win].winfixbuf
        if fixed then vim.wo[ph.win].winfixbuf = false end
        vim.api.nvim_win_set_buf(ph.win, newbuf)
        if fixed then vim.wo[ph.win].winfixbuf = true end
      end
    end
  end
end

-- --- Messages ---
--
-- Errors/warnings are highlighted messages recorded in :messages, the
-- exact echomsg-with-highlight the Vimscript used, and crucially
-- non-aborting (unlike vim.notify at ERROR, which raises under
-- nvim_exec2). Plain command output uses history=false, like the old
-- `echo`.

function echo_error(msg)
  vim.api.nvim_echo({ { msg, 'ErrorMsg' } }, true, {})
end

function echo_warn(msg)
  vim.api.nvim_echo({ { msg, 'WarningMsg' } }, true, {})
end

function echo_output(text)
  vim.api.nvim_echo({ { text } }, false, {})
end

setup()
