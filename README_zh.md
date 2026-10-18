> **语言**：本文档为简体中文翻译，[点此查看英文原版 README.md](README.md)。

<p align="center">
  <img width=400px src="https://user-images.githubusercontent.com/52205/147996902-9829bd3f-cd33-466e-833e-49a6f3ebd623.png" alt="image" />
</p>
<p align="center">
  <a href="https://github.com/dandavison/delta/actions">
    <img src="https://github.com/dandavison/delta/workflows/Continuous%20Integration/badge.svg" alt="CI">
  </a>
  <a href="https://coveralls.io/github/dandavison/delta?branch=main">
    <img src="https://coveralls.io/repos/github/dandavison/delta/badge.svg?branch=main" alt="Coverage Status">
  </a>
  <a href="https://gitter.im/dandavison-delta/community?utm_source=badge&amp;utm_medium=badge&amp;utm_campaign=pr-badge">
    <img src="https://badges.gitter.im/dandavison-delta/community.svg" alt="Gitter">
  </a>
</p>

## 快速开始

[安装它](https://dandavison.github.io/delta/installation.html)（在多数包管理器中该包名为 "git-delta"，但可执行文件就叫 `delta`），并将以下内容添加到你的 `~/.gitconfig`：

```gitconfig
[core]
    pager = delta

[interactive]
    diffFilter = delta --color-only

[delta]
    navigate = true  # use n and N to move between diff sections
    dark = true      # or light = true, or omit for auto-detection

[merge]
    conflictStyle = zdiff3
```

或者运行：

```sh
git config --global core.pager delta
git config --global interactive.diffFilter 'delta --color-only'
git config --global delta.navigate true
git config --global delta.dark true  # or `delta.light true`, or omit for auto-detection
git config --global merge.conflictStyle zdiff3
```

Delta 功能众多且极具可定制性；请查看 `delta -h`（简短帮助）或 `delta --help`（完整手册），或[在线用户手册](https://dandavison.github.io/delta/)。

## 功能特性

- 语言语法高亮，使用与 [bat](https://github.com/sharkdp/bat#readme) 相同的语法高亮主题
- 基于 Levenshtein 编辑推断算法的词级 diff 高亮
- 支持自动换行的并排视图
- 行号
- `n` 和 `N` 快捷键，用于在大 diff 的文件之间、以及在 `log -p` 视图的多个 diff 之间移动（`--navigate`）
- 改进的合并冲突显示
- 改进的 `git blame` 显示（语法高亮；`--hyperlinks` 将提交格式化为指向代码托管平台等处的链接。支持的托管平台有：GitHub、GitLab、SourceHut、Codeberg）
- 对来自 `rg`、`git grep`、`grep` 等的 grep 输出进行语法高亮
- 支持 Git 的 `--color-moved` 特性。
- 可以直接从 diff 中复制代码（默认会移除 `-/+` 标记）。
- `diff-highlight` 和 `diff-so-fancy` 模拟模式
- 提交哈希可以被格式化为指向托管平台页面的终端 [超链接](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda)（`--hyperlinks`）。
  文件路径同样可以格式化为超链接，以便在你的操作系统中打开。
- 可样式化的方框/线条装饰，用以突出提交、文件和 hunk 头部区块。
- 支持为 20 多个可样式化元素设置样式字符串（前景色、背景色、字体属性），使用与 git 相同的颜色/样式语言
- 除了 git 输出之外，也能处理传统的统一 diff（unified diff）输出
- 自动检测浅色/深色终端背景

## 一款为 git、diff 与 grep 输出做语法高亮的分页器

代码在不断演进，而我们都要花时间研读 diff。Delta 的目标是让这件事既高效又赏心悦目：它允许你对 diff 的布局与样式做大幅改动，同时也能让你尽可能贴近默认的 git/diff 输出。

<table align="center">
  <tr>
    <td>
      <img width=400px src="https://user-images.githubusercontent.com/52205/86275526-76792100-bba1-11ea-9e78-6be9baa80b29.png" alt="image" />
      <br>
      <p align="center"><sub>delta with <code>line-numbers</code> activated</sub></p>
    </td>
  </tr>
</table>

<table align="center">
  <tr>
    <td>
      <img width=800px src="https://user-images.githubusercontent.com/52205/87230973-412eb900-c381-11ea-8aec-cc200290bd1b.png" alt="image" />
      <br>
      <p align="center"><sub>delta with <code>side-by-side</code> and <code>line-numbers</code> activated</sub></p>
    </td>
  </tr>
</table>

当 git 配置为使用 delta 时，`git show` 可以呈现如下效果：

<br>

<table align="center">
  <tr>
    <td>
      <img width=500px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/81058545-a5725f80-8e9c-11ea-912e-d21954586a44.png"
           alt="image" />
    </td>
    <td>
      <img width=500px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/81058911-6abcf700-8e9d-11ea-93be-e212824ec03d.png"
           alt="image" />
    </td>
  </tr>
  <tr>
    <td>
      "Dracula" 主题
    </td>
    <td>
      "GitHub" 主题
    </td>
  </tr>
</table>

<br>
<br>

### 语法高亮主题

**凡是 [bat](https://github.com/sharkdp/bat/) 可用的语法高亮配色主题，delta 全部支持：**

<br>
<table>
  <tr>
    <td>
      <img width=400px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/149431273-e3ad049d-771e-4186-869d-0e57967958a6.png"
           alt="image" />
    </td>
    <td>
      <img width=400px style="border: 1px solid black"
           src="https://user-images.githubusercontent.com/52205/149431419-48836001-2afc-4fd0-97ad-561a69b71db7.png"
           alt="image" />
    </td>
  </tr>
  <tr>
    <td>
      <code>delta --show-syntax-themes --dark</code>
    </td>
    <td>
      <code>delta --show-syntax-themes --light</code>
    </td>
  </tr>
</table>

<br>

<a name="side-by-side-view-1"></a>

### 并排视图

[[用户手册](https://dandavison.github.io/delta/side-by-side-view.html)]

```gitconfig
[delta]
    side-by-side = true
```

默认情况下，并排视图会启用行号，并且左右两个面板都带有语法高亮：[[配置](#side-by-side-view-1)]

<table><tr><td><img width=800px src="https://user-images.githubusercontent.com/52205/87230973-412eb900-c381-11ea-8aec-cc200290bd1b.png" alt="image" /></td></tr></table>

并排视图会自动换行处理过长的行：

<table><tr><td><img width=600px src="https://user-images.githubusercontent.com/52205/139064537-f8479504-16d3-429a-b4f6-d0122438adaa.png" alt="image" /></td></tr></table>

### 行号

[[用户手册](https://dandavison.github.io/delta/line-numbers.html)]

```gitconfig
[delta]
    line-numbers = true
```

<table><tr><td><img width=400px src="https://user-images.githubusercontent.com/52205/86275526-76792100-bba1-11ea-9e78-6be9baa80b29.png" alt="image" /></td></tr></table>

### 合并冲突

[[用户手册](https://dandavison.github.io/delta/merge-conflicts.html)]

<table><tr><td><img width=500px src="https://user-images.githubusercontent.com/52205/144783121-bb549100-69d8-41b8-ac62-1704f1f7b43e.png" alt="image" /></td></tr></table>

### Git blame

[[用户手册](https://dandavison.github.io/delta/git-blame.html)]

<table><tr><td><img width=600px src="https://user-images.githubusercontent.com/52205/141891376-1fdb87dc-1d9c-4ad6-9d72-eeb19a8aeb0b.png" alt="image" /></td></tr></table>

### Ripgrep、git grep

[[用户手册](https://dandavison.github.io/delta/grep.html)]

<table><tr><td>
<img width="600px" alt="image" src="https://github.com/dandavison/open-in-editor/assets/52205/d203d380-5acb-4296-aeb9-e38c73d6c27f">
</td></tr></table>

### 安装与使用

请参阅[用户手册](https://dandavison.github.io/delta/)以及 `delta --help`。

[![Live demo by Demoshell](https://build.demoshell.com/v1/embed/badge.svg)](https://build.demoshell.com/launch?snapshot=demoshell%2Ftui%3Adelta)

### 维护者

- [@dandavison](https://github.com/dandavison)
- [@th1000s](https://github.com/th1000s)
