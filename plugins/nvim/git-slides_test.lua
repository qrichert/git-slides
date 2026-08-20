-- Headless-Neovim regression tests for the plugin.
--
-- Run from the repo root, with the binary on $PATH:
--   nvim --headless -u NONE -l plugins/nvim/git-slides_test.lua
--
-- One file, beside the plugin it tests: git-slides is a Rust CLI, so the editor
-- apparatus stays contained, not scattered like a plugin repo. The plugin is a
-- single self-contained file, so tests just `dofile` it (after setting any
-- load-time config). The main suite runs in-process (with cleanup between
-- tests); the load-time-config cases each need a fresh registration, so this
-- file re-spawns itself for them (GIT_SLIDES_TEST_CASE).

local repo = vim.fn.getcwd()
vim.g.mapleader = ',' -- fixed so mapping LHS resolve predictably

local is_windows = vim.fn.has('win32') == 1

-- Isolate every git invocation in this process (the fixture's git-slides
-- start and the plugin's own git calls, not just the git() helper) from
-- the developer's global/system git config.
vim.env.GIT_CONFIG_GLOBAL = '/dev/null'
vim.env.GIT_CONFIG_SYSTEM = '/dev/null'

-- --- Harness ---------------------------------------------------------------

local passed, failed = 0, 0
local cleanup -- defined among the helpers; the harness runs it after every test

local function eq(got, want, msg)
  if got ~= want then
    error(string.format('%s\n  expected: %s\n  got:      %s',
      msg or 'eq failed', vim.inspect(want), vim.inspect(got)), 2)
  end
end

local function ok(cond, msg)
  if not cond then error(msg or 'expected truthy', 2) end
end

-- cleanup() runs here, after the pcall, so a failing test can't leak its
-- windows/buffers/cwd into the next one and cascade spurious failures.
local function test(name, fn)
  local passed_ok, err = pcall(fn)
  cleanup()
  if passed_ok then
    passed = passed + 1
    print('  ok   ' .. name)
  else
    failed = failed + 1
    print('  FAIL ' .. name)
    print('       ' .. tostring(err):gsub('\n', '\n       '))
  end
end

local function finish(extra_failed)
  local total_failed = failed + (extra_failed or 0)
  io.write(string.format('\n%d passed, %d failed\n', passed, total_failed))
  io.flush()
  os.exit(total_failed == 0 and 0 or 1)
end

-- --- Fixture ---------------------------------------------------------------

-- Isolated so a CI runner with no global git identity/config still commits.
local GIT_ENV = {
  GIT_CONFIG_GLOBAL = '/dev/null',
  GIT_CONFIG_SYSTEM = '/dev/null',
  GIT_AUTHOR_NAME = 'git-slides test',
  GIT_AUTHOR_EMAIL = 'test@example.com',
  GIT_COMMITTER_NAME = 'git-slides test',
  GIT_COMMITTER_EMAIL = 'test@example.com',
}

local function git(dir, ...)
  local cmd = { 'git', '-C', dir }
  vim.list_extend(cmd, { ... })
  local res = vim.system(cmd, { text = true, env = GIT_ENV }):wait()
  if res.code ~= 0 then
    error('git ' .. table.concat({ ... }, ' ') .. ' failed:\n' .. (res.stderr or ''))
  end
  return res.stdout or ''
end

local function write_file(path, content)
  local fd = assert(io.open(path, 'w'))
  fd:write(content)
  fd:close()
end

-- Build a temp repo from a list of commits and start the presentation. Each
-- commit is { add = { [name] = content }, remove = { name, ... } }. Leaves
-- Neovim's cwd in the repo, checked out at slide 1. `root_dir` overrides the
-- location (used by the newline-in-root case).
local function new_repo(commits, root_dir)
  local dir = root_dir or vim.fn.tempname()
  vim.fn.mkdir(dir, 'p')
  git(dir, 'init', '-q')

  local hashes = {}
  for i, commit in ipairs(commits) do
    for name, content in pairs(commit.add or {}) do
      write_file(dir .. '/' .. name, content)
      git(dir, 'add', '--', name)
    end
    for _, name in ipairs(commit.remove or {}) do
      git(dir, 'rm', '-q', '--', name)
    end
    git(dir, 'commit', '-q', '-m', 'slide ' .. i)
    hashes[i] = vim.trim(git(dir, 'rev-parse', 'HEAD'))
  end

  local res = vim.system({ 'git-slides', 'start' }, { cwd = dir, text = true }):wait()
  if res.code ~= 0 then
    error('git-slides start failed:\n' .. (res.stdout or '') .. (res.stderr or ''))
  end

  vim.fn.chdir(dir)
  return { dir = dir, hashes = hashes }
end

-- --- Helpers ---------------------------------------------------------------

local function head(dir)
  return vim.trim(git(dir, 'rev-parse', 'HEAD'))
end

local function first_line(buf)
  return vim.api.nvim_buf_get_lines(buf, 0, 1, false)[1]
end

local function open(path)
  local buf = vim.fn.bufadd(path) -- bufadd tolerates any path (e.g. newlines)
  vim.fn.bufload(buf)
  vim.api.nvim_set_current_buf(buf)
  return buf
end

local function feed(keys)
  vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes(keys, true, false, true), 'x', false)
end

local function mapped(lhs)
  return next(vim.fn.maparg(lhs, 'n', false, true)) ~= nil
end

local function messages()
  return vim.api.nvim_exec2('messages', { output = true }).output
end

local function capture_echo(fn)
  local calls = {}
  local nvim_echo = vim.api.nvim_echo
  vim.api.nvim_echo = function(chunks, history)
    local parts = {}
    for _, chunk in ipairs(chunks) do table.insert(parts, chunk[1]) end
    table.insert(calls, { text = table.concat(parts), history = history })
  end
  local call_ok, err = pcall(fn)
  vim.api.nvim_echo = nvim_echo
  if not call_ok then error(err, 0) end
  return calls
end

-- Reset editor state between in-process tests. Clears winfixbuf first so
-- windows aren't pinned when their buffers are wiped.
function cleanup()
  for _, w in ipairs(vim.api.nvim_list_wins()) do
    pcall(function() vim.wo[w].winfixbuf = false end)
  end
  vim.cmd('silent! helpclose')
  vim.cmd('silent! only')
  vim.cmd('silent! %bwipeout!')
end

local function load_plugin()
  dofile(repo .. '/plugins/nvim/plugin/git-slides.lua')
end

-- --- Main suite (in-process) -----------------------------------------------

local function main_suite()
  -- The gate on `revert`: a modified file must refresh in every window. This
  -- is the only test exercising nvim_buf_call + :edit! on changed content, so
  -- a 0.10 textlock regression that no-ops the reload fails here, not in CI.
  test('modified file refreshes in every window', function()
    local r = new_repo({
      { add = { ['a.txt'] = 'v1\n' } },
      { add = { ['a.txt'] = 'v2\n' } },
    })
    local buf = open(r.dir .. '/a.txt')
    vim.cmd('split')
    eq(#vim.fn.win_findbuf(buf), 2, 'open in two windows')
    eq(first_line(buf), 'v1', 'starts at slide 1')
    vim.cmd('GitSlidesGo 2')
    eq(#vim.fn.win_findbuf(buf), 2, 'both windows still show the buffer')
    eq(first_line(buf), 'v2', 'buffer reloaded to v2')
    eq(vim.bo[buf].modified, false, "'modified' clear after reload")
  end)

  test('deleted file drops to a placeholder in every window', function()
    local r = new_repo({
      { add = { ['keep.txt'] = 'k\n', ['gone.txt'] = 'g\n' } },
      { add = { ['keep.txt'] = 'k2\n' }, remove = { 'gone.txt' } },
    })
    local buf = open(r.dir .. '/gone.txt')
    vim.cmd('split')
    eq(#vim.api.nvim_list_wins(), 2, 'two windows before')
    vim.cmd('GitSlidesGo 2')
    eq(#vim.api.nvim_list_wins(), 2, 'layout preserved')
    ok(not vim.api.nvim_buf_is_valid(buf), 'original buffer wiped')
    local wins = vim.api.nvim_list_wins()
    local b1 = vim.api.nvim_win_get_buf(wins[1])
    local b2 = vim.api.nvim_win_get_buf(wins[2])
    ok(b1 ~= b2, 'distinct placeholders per window')
    eq(vim.bo[b1].buftype, 'nofile', 'placeholder 1 is nofile')
    eq(vim.bo[b2].buftype, 'nofile', 'placeholder 2 is nofile')
    eq(vim.bo[b1].modifiable, false, 'placeholder 1 is nonmodifiable')
    eq(vim.bo[b2].modifiable, false, 'placeholder 2 is nonmodifiable')
  end)

  test('winfixbuf window survives a drop and is restored', function()
    local r = new_repo({
      { add = { ['keep.txt'] = 'k\n', ['gone.txt'] = 'g\n' } },
      { add = { ['keep.txt'] = 'k2\n' }, remove = { 'gone.txt' } },
    })
    local buf = open(r.dir .. '/gone.txt')
    local win = vim.api.nvim_get_current_win()
    vim.wo[win].winfixbuf = true
    vim.cmd('GitSlidesGo 2')
    ok(not vim.api.nvim_buf_is_valid(buf), 'original wiped')
    eq(vim.bo[vim.api.nvim_win_get_buf(win)].buftype, 'nofile', 'placeholder installed')
    eq(vim.wo[win].winfixbuf, true, 'winfixbuf restored')
  end)

  test('reload of a hidden buffer spares a bufhidden=wipe current buffer', function()
    local r = new_repo({
      { add = { ['a.txt'] = 'v1\n' } },
      { add = { ['a.txt'] = 'v2\n' } },
    })
    local a = open(r.dir .. '/a.txt')
    vim.cmd('enew') -- `a` becomes hidden (default 'hidden' is on)
    vim.bo.bufhidden = 'wipe'
    local scratch = vim.api.nvim_get_current_buf()
    vim.cmd('GitSlidesGo 2')
    ok(vim.api.nvim_buf_is_valid(scratch), 'wipe-on-hide current buffer survived')
    eq(vim.api.nvim_get_current_buf(), scratch, 'still on the scratch buffer')
    ok(vim.api.nvim_buf_is_valid(a), 'hidden file buffer still valid')
    eq(first_line(a), 'v2', 'hidden buffer reloaded')
  end)

  test('navigation guards unsaved buffers; bang forces', function()
    local r = new_repo({
      { add = { ['a.txt'] = 'v1\n' } },
      { add = { ['a.txt'] = 'v2\n' } },
    })
    local buf = open(r.dir .. '/a.txt')
    vim.api.nvim_buf_set_lines(buf, 0, -1, false, { 'dirty' })
    eq(vim.bo[buf].modified, true, 'buffer is modified')
    vim.cmd('GitSlidesGo 2') -- no bang: abort
    eq(head(r.dir), r.hashes[1], 'still on slide 1 after aborted nav')
    eq(vim.bo[buf].modified, true, 'buffer left modified')
    vim.cmd('GitSlidesGo! 2') -- bang: discard + reload
    eq(head(r.dir), r.hashes[2], 'moved to slide 2 with bang')
    eq(first_line(buf), 'v2', 'discarded edits, reloaded from disk')
    eq(vim.bo[buf].modified, false, "'modified' clear after forced reload")
  end)

  -- The gate on the fingerprint-scoped reload: only the file the slide
  -- touched refreshes, so an unrelated buffer keeps its cursor and undo (a
  -- bumped changedtick would betray a needless :edit!).
  test('a mutating command reloads only the files it changed', function()
    local r = new_repo({
      { add = { ['a.txt'] = 'a1\n', ['b.txt'] = 'b\n' } },
      { add = { ['a.txt'] = 'a2\n' } }, -- slide 2 touches a.txt only
    })
    local a = open(r.dir .. '/a.txt')
    local b = open(r.dir .. '/b.txt')
    local b_tick = vim.api.nvim_buf_get_changedtick(b)
    vim.cmd('GitSlidesGo 2')
    eq(first_line(a), 'a2', 'changed file reloaded')
    eq(vim.api.nvim_buf_get_changedtick(b), b_tick, 'unchanged file left alone')
  end)

  -- Disk can already contain the destination bytes before navigation (for
  -- example after an external edit). The checkout then leaves the fingerprint
  -- unchanged, but the stale loaded text still needs to refresh.
  test('navigation reloads a buffer that was already stale', function()
    local r = new_repo({
      { add = { ['a.txt'] = 'v1\n' } },
      { add = { ['a.txt'] = 'v2\n' } },
    })
    local buf = open(r.dir .. '/a.txt')
    write_file(r.dir .. '/a.txt', 'v2\n')
    eq(first_line(buf), 'v1', 'buffer is stale before navigation')
    eq(vim.bo[buf].modified, false, 'external edit did not modify the buffer')
    vim.cmd('GitSlidesGo 2')
    eq(first_line(buf), 'v2', 'stale buffer reloaded to destination bytes')
  end)

  -- A failed command that mutated nothing must not force-reload: the bang
  -- bypasses the guard, but with no on-disk change there is nothing to sync,
  -- so unsaved edits survive.
  test('a failed command that changed nothing does not reload buffers', function()
    local r = new_repo({
      { add = { ['a.txt'] = 'v1\n' } },
      { add = { ['a.txt'] = 'v2\n' } },
    })
    local buf = open(r.dir .. '/a.txt')
    vim.api.nvim_buf_set_lines(buf, 0, -1, false, { 'dirty' })
    vim.cmd('GitSlidesStart!') -- already presenting: errors, mutates nothing
    eq(vim.bo[buf].modified, true, 'a no-op command spares unsaved edits')
    eq(first_line(buf), 'dirty', 'buffer not force-reloaded')
  end)

  -- The bang's contract is to discard unsaved edits and show the checked-out
  -- slide, even for a file the slide leaves byte-identical (fingerprint alone
  -- would skip it).
  test('a forced nav discards a dirty buffer even if its file is unchanged', function()
    local r = new_repo({
      { add = { ['a.txt'] = 'same\n', ['b.txt'] = 'b1\n' } },
      { add = { ['b.txt'] = 'b2\n' } }, -- slide 2 leaves a.txt untouched
    })
    local buf = open(r.dir .. '/a.txt')
    vim.api.nvim_buf_set_lines(buf, 0, -1, false, { 'dirty' })
    vim.cmd('GitSlidesGo! 2') -- bang: discard unsaved edits
    eq(head(r.dir), r.hashes[2], 'navigated to slide 2')
    eq(vim.bo[buf].modified, false, 'forced nav cleared the modification')
    eq(first_line(buf), 'same', 'discarded edits, back to on-disk content')
  end)

  -- The fingerprint is over raw bytes: a checkout swapping a NUL for an LF at
  -- the same offset is a real change, not a no-op a line-oriented read would
  -- hash identically.
  test('a binary NUL/LF change is not mistaken for no change', function()
    local r = new_repo({
      { add = { ['bin'] = 'a\0b\n' } },
      { add = { ['bin'] = 'a\nb\n' } },
    })
    local buf = open(r.dir .. '/bin')
    local tick = vim.api.nvim_buf_get_changedtick(buf)
    vim.cmd('GitSlidesGo 2')
    ok(vim.api.nvim_buf_get_changedtick(buf) ~= tick, 'the NUL->LF change triggered a reload')
  end)

  -- A reload hook that errors (a user BufReadPost) makes :edit! fail even
  -- though the file is perfectly readable; the buffer must be kept and the
  -- error reported, never dropped as if the file had vanished.
  test('a failing reload hook keeps the buffer, does not drop it', function()
    local r = new_repo({
      { add = { ['a.txt'] = 'v1\n' } },
      { add = { ['a.txt'] = 'v2\n' } },
    })
    local buf = open(r.dir .. '/a.txt')
    local group = vim.api.nvim_create_augroup('git_slides_test_hook', { clear = true })
    vim.api.nvim_create_autocmd('BufReadPost', {
      group = group,
      buffer = buf,
      callback = function() error('boom') end,
    })
    vim.cmd('GitSlidesGo 2')
    vim.api.nvim_del_augroup_by_id(group)
    ok(vim.api.nvim_buf_is_valid(buf), 'buffer kept despite the failing reload hook')
  end)

  -- A file added mid-presentation, open when you rewind before its slide,
  -- becomes a placeholder; advancing back to a slide that has it must restore
  -- the real buffer, not leave the window blank forever.
  test('a dropped file is restored when a later slide re-adds it', function()
    local r = new_repo({
      { add = { ['a.txt'] = 'a\n' } },
      { add = { ['new.txt'] = 'hello\n' } }, -- new.txt appears at slide 2
    })
    vim.cmd('GitSlidesGo 2')
    local buf = open(r.dir .. '/new.txt')
    local win = vim.api.nvim_get_current_win()
    eq(first_line(buf), 'hello', 'new.txt open at slide 2')
    vim.cmd('GitSlidesGo 1') -- new.txt vanishes -> placeholder
    ok(not vim.api.nvim_buf_is_valid(buf), 'file buffer dropped at slide 1')
    eq(vim.bo[vim.api.nvim_win_get_buf(win)].buftype, 'nofile', 'placeholder shown')
    vim.cmd('GitSlidesGo 2') -- new.txt returns -> revive
    local revived = vim.api.nvim_win_get_buf(win)
    eq(vim.bo[revived].buftype, '', 'file buffer restored, not a placeholder')
    eq(first_line(revived), 'hello', 'restored on-disk content')
  end)

  -- BufReadPost errors after the restored file has been read. They must be
  -- reported without aborting navigation or leaving the usable loaded buffer
  -- hidden behind its placeholder.
  test('a failing revival hook reports without aborting', function()
    local r = new_repo({
      { add = { ['a.txt'] = 'a\n' } },
      { add = { ['new.txt'] = 'hello\n' } },
    })
    vim.cmd('GitSlidesGo 2')
    open(r.dir .. '/new.txt')
    local win = vim.api.nvim_get_current_win()
    vim.cmd('GitSlidesGo 1')
    eq(vim.bo[vim.api.nvim_win_get_buf(win)].buftype, 'nofile', 'placeholder shown')

    local group = vim.api.nvim_create_augroup('git_slides_test_revive_hook', { clear = true })
    vim.api.nvim_create_autocmd('BufReadPost', {
      group = group,
      pattern = 'new.txt',
      callback = function() error('revive boom') end,
    })
    local command_ok = pcall(vim.cmd, 'GitSlidesGo 2')
    vim.api.nvim_del_augroup_by_id(group)

    ok(command_ok, 'navigation did not propagate the load error')
    local revived = vim.api.nvim_win_get_buf(win)
    eq(vim.bo[revived].buftype, '', 'loaded file replaced the placeholder')
    eq(first_line(revived), 'hello', 'restored content remains visible')
    ok(messages():find('git%-slides: could not reload .*new%.txt'), 'load error was reported')
  end)

  -- An independent repo nested in the outer worktree must be off-limits: a
  -- forced outer nav must not discard its unsaved edits just because its path
  -- sits lexically under the outer root.
  test('a forced nav spares dirty buffers in a nested repository', function()
    local r = new_repo({
      { add = { ['a.txt'] = 'a1\n' } },
      { add = { ['a.txt'] = 'a2\n' } },
    })
    local nested = r.dir .. '/vendor'
    vim.fn.mkdir(nested, 'p')
    git(nested, 'init', '-q')
    write_file(nested .. '/lib.txt', 'lib\n')
    git(nested, 'add', '--', 'lib.txt')
    git(nested, 'commit', '-q', '-m', 'lib')
    local buf = open(nested .. '/lib.txt')
    vim.api.nvim_buf_set_lines(buf, 0, -1, false, { 'dirty' })
    vim.cmd('GitSlidesGo! 2') -- outer nav; must not touch the nested buffer
    eq(vim.bo[buf].modified, true, 'nested-repo edits preserved')
    eq(first_line(buf), 'dirty', 'nested buffer not force-reloaded')
  end)

  -- Recursive checkout updates active submodule worktrees. Their open buffers
  -- must join the guard/reload set, while the same submodule remains excluded
  -- when recursion is disabled.
  test('recursive checkout reloads submodule buffers', function()
    local child = vim.fn.tempname()
    vim.fn.mkdir(child, 'p')
    git(child, 'init', '-q')
    write_file(child .. '/lib.txt', 'v1\n')
    git(child, 'add', '--', 'lib.txt')
    git(child, 'commit', '-q', '-m', 'submodule 1')
    local child_v1 = vim.trim(git(child, 'rev-parse', 'HEAD'))
    write_file(child .. '/lib.txt', 'v2\n')
    git(child, 'commit', '-q', '-am', 'submodule 2')
    local child_v2 = vim.trim(git(child, 'rev-parse', 'HEAD'))

    local outer = vim.fn.tempname()
    vim.fn.mkdir(outer, 'p')
    git(outer, 'init', '-q')
    git(outer, '-c', 'protocol.file.allow=always', 'submodule', 'add', '-q', child, 'vendor')
    git(outer .. '/vendor', 'checkout', '-q', child_v1)
    write_file(outer .. '/a.txt', 'outer 1\n')
    git(outer, 'add', '--', 'a.txt', 'vendor')
    git(outer, 'commit', '-q', '-m', 'slide 1')
    git(outer .. '/vendor', 'checkout', '-q', child_v2)
    write_file(outer .. '/a.txt', 'outer 2\n')
    git(outer, 'add', '--', 'a.txt', 'vendor')
    git(outer, 'commit', '-q', '-m', 'slide 2')
    git(outer, 'config', 'submodule.recurse', 'true')

    local res = vim.system({ 'git-slides', 'start' }, { cwd = outer, text = true }):wait()
    eq(res.code, 0, 'git-slides start with recursive submodules')
    vim.fn.chdir(outer)
    local buf = open(outer .. '/vendor/lib.txt')
    eq(vim.fn.filereadable(outer .. '/vendor/.git'), 1, 'submodule has a .git file')
    eq(first_line(buf), 'v1', 'submodule starts at slide 1 revision')

    git(outer, 'config', 'submodule.recurse', 'false')
    vim.api.nvim_buf_set_lines(buf, 0, -1, false, { 'dirty' })
    vim.cmd('GitSlidesGo! 2')
    eq(first_line(buf), 'dirty', 'non-recursed submodule buffer was spared')
    vim.api.nvim_buf_call(buf, function() vim.cmd('silent edit!') end)

    git(outer, 'config', 'submodule.recurse', 'true')
    vim.cmd('GitSlidesGo 1')
    vim.cmd('GitSlidesGo 2')
    eq(vim.trim(git(outer .. '/vendor', 'rev-parse', 'HEAD')), child_v2,
      'recursive checkout advanced the submodule')
    eq(first_line(buf), 'v2', 'submodule buffer reloaded to slide 2 revision')
  end)

  -- The CLI can report a failed stash and still exit successfully when the
  -- dirty path does not block checkout. Neovim must not hide that stderr.
  test('successful navigation reports stderr as a warning', function()
    local r = new_repo({
      { add = { ['a.txt'] = 'same\n', ['b.txt'] = 'v1\n' } },
      { add = { ['b.txt'] = 'v2\n' } },
    })
    write_file(r.dir .. '/a.txt', 'dirty\n')
    write_file(r.dir .. '/.git/refs/stash.lock', '')
    open(r.dir .. '/b.txt')
    vim.cmd('GitSlidesGo 2')
    eq(head(r.dir), r.hashes[2], 'checkout succeeded despite the stash failure')
    ok(messages():find('git%-slides: error: Could not stash uncommitted changes%.'),
      'successful-process stderr was reported')
  end)

  -- A child killed by a signal returns { code = 0, signal = 15 } on POSIX;
  -- that is a failure, so a bang must not discard edits over it. (No signals
  -- to exercise on Windows.)
  if not is_windows then
    test('a signal-killed command is treated as a failure', function()
      local r = new_repo({
        { add = { ['a.txt'] = 'v1\n' } },
        { add = { ['a.txt'] = 'v2\n' } },
      })
      local script = r.dir .. '/selfkill.sh'
      write_file(script, '#!/bin/sh\nkill -TERM $$\n')
      vim.fn.setfperm(script, 'rwxr-xr-x')
      vim.g.git_slides_executable = script
      local buf = open(r.dir .. '/a.txt')
      vim.api.nvim_buf_set_lines(buf, 0, -1, false, { 'dirty' })
      vim.cmd('GitSlidesGo! 2')
      vim.g.git_slides_executable = nil
      eq(vim.bo[buf].modified, true, 'a signal kill does not discard edits on bang')
    end)
  end

  -- Windows filesystems forbid newlines in path names, so this case can only
  -- run on POSIX.
  if not is_windows then
    test('repository root containing a newline works', function()
      local dir = vim.fn.tempname() .. '/root\nwith\nnewline'
      local r = new_repo({
        { add = { ['a.txt'] = 'v1\n' } },
        { add = { ['a.txt'] = 'v2\n' } },
      }, dir)
      local buf = open(r.dir .. '/a.txt')
      vim.cmd('GitSlidesGo 2')
      eq(first_line(buf), 'v2', 'navigated with a newline in the repo root')
    end)
  end

  test(':GitSlidesGo 3 and :3GitSlidesGo both jump to slide 3', function()
    local r = new_repo({
      { add = { ['a.txt'] = '1\n' } },
      { add = { ['a.txt'] = '2\n' } },
      { add = { ['a.txt'] = '3\n' } },
    })
    open(r.dir .. '/a.txt')
    vim.cmd('GitSlidesGo 3') -- count as argument
    eq(head(r.dir), r.hashes[3], ':GitSlidesGo 3')
    vim.cmd('GitSlidesGo 1')
    eq(head(r.dir), r.hashes[1], 'reset to slide 1')
    vim.cmd('3GitSlidesGo') -- count as prefix
    eq(head(r.dir), r.hashes[3], ':3GitSlidesGo')
  end)

  test('default <Leader>sn mapping advances a slide', function()
    local r = new_repo({
      { add = { ['a.txt'] = '1\n' } },
      { add = { ['a.txt'] = '2\n' } },
    })
    open(r.dir .. '/a.txt')
    vim.cmd('GitSlidesGo 1')
    feed(',sn') -- mapleader is ','
    eq(head(r.dir), r.hashes[2], 'mapping ran `next`')
  end)

  -- The gate on cwd(): with a non-repo buffer focused, a command must still
  -- act on the repo Neovim is cd'd into, not the focused buffer's own dir.
  test('a command from a help buffer still targets the repo cwd', function()
    local r = new_repo({
      { add = { ['a.txt'] = '1\n' } },
      { add = { ['a.txt'] = '2\n' } },
    })
    vim.cmd('GitSlidesGo 1') -- baseline: slide 1
    vim.cmd('help')          -- focus a non-repo buffer
    vim.cmd('GitSlidesGo 2') -- must still navigate the repo
    eq(head(r.dir), r.hashes[2], 'navigated the repo, not the help doc dir')
  end)

  test('list returns text under a pipe (no pager hang)', function()
    local r = new_repo({
      { add = { ['a.txt'] = '1\n' } },
      { add = { ['a.txt'] = '2\n' } },
    })
    local res = vim.system({ 'git-slides', 'list' }, { cwd = r.dir, text = true }):wait()
    eq(res.code, 0, 'list exits 0 under a pipe')
    ok((res.stdout or ''):find('1/2'), 'list output captured')
    open(r.dir .. '/a.txt')
    ok(pcall(vim.cmd, 'GitSlidesList'), ':GitSlidesList runs')
  end)

  test('successful commands show only intentional output', function()
    new_repo({
      { add = { ['a.txt'] = '1\n' } },
      { add = { ['a.txt'] = '2\n' } },
    })

    local navigation = capture_echo(function() vim.cmd('GitSlidesGo 2') end)
    eq(#navigation, 0, 'navigation stdout is silent')

    local status = capture_echo(function() vim.cmd('GitSlidesStatus') end)
    eq(#status, 1, 'status still displays its output')
    ok(status[1].text:find('2/2', 1, true), 'status output is preserved')

    local list = capture_echo(function() vim.cmd('GitSlidesList') end)
    eq(#list, 1, 'list still displays its output')
    ok(list[1].text:find('2/2', 1, true), 'list output is preserved')

    local stopped = capture_echo(function() vim.cmd('GitSlidesStop') end)
    eq(#stopped, 1, 'stop displays one note')
    eq(stopped[1].text, 'git-slides: presentation ended', 'stop note')

    local started = capture_echo(function() vim.cmd('GitSlidesStart') end)
    eq(#started, 1, 'start displays one note')
    eq(started[1].text, 'git-slides: presentation started', 'start note')
  end)
end

-- --- Load-time-config cases (each in its own child) ------------------------

local function no_default_mappings_suite()
  test('no default maps when disabled; <Plug> maps remain', function()
    ok(not mapped(',ss'), 'default <Leader>ss not mapped')
    ok(mapped('<Plug>(GitSlidesStart)'), '<Plug> map still registered')
  end)
end

local function zero_default_mappings_suite()
  test('numeric zero keeps default maps', function()
    ok(mapped(',ss'), 'default <Leader>ss remains mapped')
    ok(mapped('<Plug>(GitSlidesStart)'), '<Plug> map remains registered')
  end)
end

local function custom_prefix_suite()
  test('custom mapping prefix is honored', function()
    ok(mapped(',xs'), '<Leader>x prefix maps Start')
    ok(not mapped(',ss'), 'default prefix unused')
  end)
end

-- --- Orchestration ---------------------------------------------------------

local case = os.getenv('GIT_SLIDES_TEST_CASE')

if case == 'no_default_mappings' then
  vim.g.git_slides_no_default_mappings = true
  load_plugin()
  no_default_mappings_suite()
  finish()
elseif case == 'zero_default_mappings' then
  vim.g.git_slides_no_default_mappings = 0
  load_plugin()
  zero_default_mappings_suite()
  finish()
elseif case == 'custom_prefix' then
  vim.g.git_slides_mapping_prefix = '<Leader>x'
  load_plugin()
  custom_prefix_suite()
  finish()
else
  load_plugin()
  main_suite()

  -- The config cases need a fresh registration, so run them in child
  -- Neovims and fold their pass/fail tallies into the totals.
  for _, c in ipairs({ 'no_default_mappings', 'zero_default_mappings', 'custom_prefix' }) do
    print('\n-- case: ' .. c)
    local res = vim.system(
      { vim.v.progpath, '--headless', '-u', 'NONE', '-l', 'plugins/nvim/git-slides_test.lua' },
      { cwd = repo, text = true, env = { GIT_SLIDES_TEST_CASE = c } }
    ):wait()
    io.write(res.stdout or '')
    io.write(res.stderr or '')
    local cp, cf = (res.stdout or ''):match('(%d+) passed, (%d+) failed')
    if cp then
      passed = passed + tonumber(cp)
      failed = failed + tonumber(cf)
    end
    -- Count a crash (nonzero exit without a matching failure count) so
    -- the totals never claim success when a child died.
    if res.code ~= 0 and (not cf or tonumber(cf) == 0) then
      failed = failed + 1
    end
  end

  finish()
end
