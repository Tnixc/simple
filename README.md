# Simple Build tool

A simple build tool that assembles a static site from components, templates, and
data. I used it to build v6 of https://tnixc.space

## Todo List

### Core

- [ ] LSP (and check command)
- [ ] Switch to using a .config file for configuration
- [ ] Implement caching based on file hashes to avoid unnecessary rebuilds
- [ ] Handle port collisions in dev server

### Components and Templates

- [ ] Improve Template parsing strictness
- [ ] Implement type safety-ish warnings for template-data mismatches
- [ ] Add CSS scoping in components (waiting for @scope general support)
- [ ] Provide warnings for unused and unfound components

### Data

- [ ] Resolve dual sources of truth for Markdown frontmatter in blog posts
      (can't fix without proper Markdown parsing into entries)
- [ ] Improve JSON parsing error handling

### Markdown and Content

- [ ] Implement OG Image and meta generation, especially for Markdown posts

### Errors/Logging

- [ ] Exact file and line in error messages

### Perf

- [ ] Implement selective component caching to reduce disk reads
- [ ] Conduct and analyze more extensive speed tests

### Syntax/Parsing

- [ ] Improve escaping for special characters ({}, "", etc.) (needed?)

### Testing

- [ ] Unit tests and end-to-end tests

### Done

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
├── components
│   ├── Common
│   │   └── Pill.component.html
│   └── PostLayout.component.html
├── data
│   └── Projects.data.json
├── pages
│   ├── content
│   │   └── about.html
│   └── index.html
├── public
│   ├── assets
│   │   └── image.webp
│   ├── favicon.png
│   ├── fonts
│   │   └── Inter.woff2
│   ├── input.css
│   └── output.css
└── templates
    └── Projects.template.html
```

To use the above, you would run the following command, where target contains a
folder `src`.

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
`src/data/Name.data.json` to populate instances of
`src/templates/Name.template.html`

Below is an example of a template file:

```html
<!-- Posts.template.json -->
<a class="bg-neutral-400/20 p-4 block post relative group" href="${link}">
  <h1 class="font-grotesk text-2xl">${title}</h1>
  <p class="text-neutral-700 pt-1">Published ${date}</p>
  <p class="text-neutral-700 pt-3">${description}</p>
</a>
```

Note the `${}` items. They will match with the following in `Posts.data.json`

```json
[
    {
        "title": "Title",
        "description": "desc",
        "link": "./content/simple.html",
        "date": "dd/mm/yyyy"
    },
    {...}
]
```

The `data.json` file must contain an array of objects with the keys for the
template.

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

### Template entries

You can also use the template syntax to render entries from files.

Take this `src/data/Posts.data.json`:

```json
[
    {
        "link": "./content/simple.html",
        "date": "dd/mm/yyyy",
        "--entry-path": "content/simple.md"
        "--result-path": "content/simple.html"
    },
    {...}
]
```

The keys starting with `--` are special. The `entry-path` key is the path to the
file to be rendered, the base path is `src/data`. The `result-path` is the path
to the file to be written to.

If the key `--entry-path` or `--result-path` is present, the program will look
for the file `src/templates/Posts.frame.html` and render the entry using that
frame file. Inside that frame file, the string `${--content}` will be inlined as
rendered markdown if the `--entry-path` is a markdown file, or the file contents
if it is not a markdown file. Note that the key-value pairs in the object will
work on both the rendered entry file AND the template file. The idea is that you
can use one template for both the table of contents and the individual pages.

**If either of these keys are present, the other one must also be present.**

### Syntax highlighting

Syntax highlighting is supported. It outputs to codeblocks with the syntect
highlighting classes. There's tools to convert vscode themes to .tmTheme(textmate theme) files
into the css. I made a [web app](https://tm-theme2css.vercel.app/) for the process.

## Naming

Components, templates, and data must following `CamalCase`, not contain spaces,
and cannot start with a number. They can contain underscores.
