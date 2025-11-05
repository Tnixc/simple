# Simple Frontmatter Example

This example demonstrates how to use YAML frontmatter in markdown files with Simple.

## Structure

```
example/
├── src/
│   ├── data/
│   │   ├── Posts.data.toml              # Configuration listing files
│   │   └── Posts/
│   │       ├── my-first-post.md         # Blog post with frontmatter
│   │       ├── getting-started.md       # Another blog post
│   │       └── advanced-features.md     # Yet another post
│   ├── templates/
│   │   ├── Posts.template.html          # Template for post cards
│   │   └── Posts.frame.html             # Frame for individual post pages
│   ├── pages/
│   │   └── index.html                   # Main page that lists all posts
│   └── public/
│       └── (static assets go here)
```

**Important**: The TOML configuration file (`Posts.data.toml`) goes in `src/data/`, while the markdown files go in `src/data/Posts/`.

## How It Works

1. **Markdown files with frontmatter**: Each `.md` file in `data/Posts/` contains YAML frontmatter with metadata:
   ```markdown
   ---
   title: My Post
   description: A great post
   date: Jan 15 2025
   author: John Doe
   ---

   # Content here...
   ```

2. **TOML configuration**: `Posts.data.toml` specifies which files to include and in what order:
   ```toml
   files = [
     "my-first-post.md",
     "getting-started.md",
     "advanced-features.md"
   ]
   ```

3. **Template file**: `Posts.template.html` defines how each post appears in the list on the index page

4. **Frame file**: `Posts.frame.html` defines the layout for individual post pages

5. **Index page**: `index.html` uses `<-Template{Posts} />` to render all posts

## Building

From the repository root:

```bash
# Build the example
./target/release/simple build example

# Or run in development mode
./target/release/simple dev example
```

The built site will be in `example/dist/`.

## What Gets Generated

- `dist/index.html` - Main page listing all posts
- `dist/content/my-first-post.html` - Individual post page
- `dist/content/getting-started.html` - Individual post page
- `dist/content/advanced-features.html` - Individual post page

## Customization

You can add any fields you want to the frontmatter:

```yaml
---
title: Required field
description: Optional
author: Optional
date: Optional
tags: Optional
readtime: Optional
custom_field: Optional
anything_you_want: Optional
---
```

All fields become available as template variables: `${title}`, `${author}`, `${custom_field}`, etc.

Only `title` is required!
