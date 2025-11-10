# Simple Build tool

> [!WARNING]
> Probably very buggy, it's very opinionated and not extensible.

A simple build tool that assembles a static site from components, templates, and
data. I used it to build v6 of https://enochlau.com

## Todo List

### Core

- [ ] Switch to using a .config file for configuration
- [ ] Implement caching based on file hashes to avoid unnecessary rebuilds

### Components and Templates

- [ ] Improve Template parsing strictness
- [ ] Implement type safety-ish warnings for template-data mismatches
- [ ] Add CSS scoping in components (waiting for @scope general support)
- [ ] Provide warnings for unused and unfound components

### Data

- [ ] Improve data parsing error handling

### Errors/Logging

- [ ] Exact file and line in error messages

### Perf

- [ ] Implement selective component caching to reduce disk reads

### Syntax/Parsing

- [ ] Improve escaping for special characters ({}, "", etc.) (needed?)

### Done

- [x] Proper error handling (removed all unwrap/expect calls)
- [x] Performance optimization with Rayon parallelization (12% faster)
- [x] Strip frontmatter from markdown before rendering
- [x] KaTeX support
- [x] Handle port collisions in dev server
- [x] Resolve dual sources of truth for Markdown frontmatter in blog posts (can't fix without proper Markdown parsing into entries)
- [x] Bi-directional editing: You can now double click on a rendered `<markdown>` element to edit it, and it's reflected in the source code.
- [x] Implement proper Markdown rendering from .md files -> template entries
- [x] Minify after build with minify_html, minifiy-js, lightningcss
- [x] Multiple errors at once
- [x] Better errs with file and more specific messages
- [x] Fix Markdown element indentation issues
- [x] Fix errors to more accurately reflect the source of the error
- [x] Made it much faster by threadding at page_handler
- [x] Enhance flexibility of Markdown syntax highlighting themes (now uses css)
- [x] Check for circular dependencies
- [x] Implement file watcher and Hot Module Reloading (HMR)
- [x] Enhance logging with color, status, etc.
- [x] Add <markdown> component
- [x] Implement commands: dev, build, new
- [x] Add support for props and slots in components
- [x] Ignore commented out lines during parsing
- [x] Improve error handling (ongoing process)
- [x] Implement templating system
- [x] Implement component system
- [x] Set up file copying from /public to /dist

# Documentation

## File structure


```
src
├── data
│   ├── a.md
│   ├── Posts
│   │   ├── advanced-features.md
│   │   ├── getting-started.md
│   │   └── my-first-post.md
│   └── Posts.data.toml
├── pages
│   └── index.html
├── public
│   ├── assets
│   │   └── image.webp
│   ├── favicon.png
│   └── styles.css
└── templates
    ├── Posts.frame.html
    └── Posts.template.html
```

To use the above, you would run the following command, where target contains a folder `src`.

```
simple <build|dev|new> /path/to/target
```

## Components

To use components in markup, do the following:

```html
<Component />
<!-- for a self closing component -->
<Layout><h1>Hello world</h1></Layout>
<!-- for a component which contains a <slot></slot> -->
```

If no content inside a slot component is provided, it will use the fallbacks
inside the `<slot>` tags. To access components in folders, use a `:` to separate
them like so:

```html
<Folder:Component />
```

### Props

To pass props to a component, use the following syntax:

```html
<Component prop="value" />
```

Which will be accessible in the component as `${prop}`.

## Templating

```html
<!-- e.g. in index.html -->
<-Template{Name} />
```

Think of this like a `for each`, where `Name` will be used to search for
`src/data/Name.data.toml` to populate instances of
`src/templates/Name.template.html`

Below is an example of a template file:

```html
<!-- Posts.template.html -->
<article class="post-card">
  <a href="${link}">
    <h2>${title}</h2>
    <p class="meta">
      <span class="date">${date}</span>
      <span class="author">by ${author}</span>
    </p>
    <p class="description">${description}</p>
  </a>
</article>
```

Note the `${}` items. These are template variables that get populated from your data source.

### Using TOML with Frontmatter (Recommended)

The recommended approach is to use TOML to specify which markdown files to include, and extract metadata from YAML frontmatter in those files.

**`src/data/Posts.data.toml`** - Specifies which files to include and their order:
```toml
# List of markdown files to include as blog posts
files = [
  "my-first-post.md",
  "getting-started.md",
  "advanced-features.md"
]
```

**`src/data/Posts/my-first-post.md`** - Markdown file with YAML frontmatter:
```markdown
---
title: My First Blog Post
description: Welcome to my new blog!
date: Jan 15 2025
author: John Doe
---

# Your Content Here

Your markdown content...
```

The frontmatter is automatically stripped before rendering and used to generate the data for templating. The following fields are auto-generated:
- `--entry-path`: Set to the markdown file path
- `--result-path`: Set to `content/{filename}.html`
- `link`: Set to `./content/{filename}.html`

All other frontmatter fields (like `title`, `description`, `date`, `author`) are available as template variables.

**Required fields:** Only `title` is required in the frontmatter.

## The `<markdown>` component

There's also a `<markdown>` component:

```html
<markdown>
  # Hello world

  <img src="image.webp" alt="alt" />
</markdown>
```

Will render out to:

```html
<h1>Hello world</h1>
<img src="image.webp" alt="alt" />
```

> [!NOTE]
> You can double click on a rendered markdown element and edit it from the web. The changes will be reflected in the source file. It is a bit flakely with escaped html entities, so try to avoid using those.

### Frame Files

When using markdown files with frontmatter as shown above, the program will look for a frame file at `src/templates/Posts.frame.html` to wrap the rendered content. Inside the frame file, the string `${--content}` will be replaced with the rendered markdown content.

The frame file can use any of the frontmatter variables (like `${title}`, `${date}`, etc.) as well as the special `${--content}` variable. This allows you to create a consistent layout for all your blog posts while keeping the content in separate markdown files.

### Syntax highlighting

Syntax highlighting is supported. It outputs to codeblocks with the syntect
highlighting classes. There's tools to convert vscode themes to .tmTheme(textmate theme) files
into the css. I made a [web app](https://tm-theme2css.vercel.app/) for the process.

## Naming

Components, templates, and data must following `CamalCase`, not contain spaces,
and cannot start with a number. They can contain underscores.
