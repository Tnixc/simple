---
title: Getting Started with Simple
description: A comprehensive guide to setting up your first Simple project
date: Jan 20 2025
author: John Doe
tags: tutorial, beginner
readtime: 5 min
---

# Getting Started with Simple

In this post, I'll walk you through setting up your first project with Simple.

## Installation

First, you'll need to install Simple. The easiest way is to use cargo:

```bash
cargo install simple-web
```

## Project Structure

A typical Simple project looks like this:

```
my-project/
├── src/
│   ├── components/      # Reusable UI components
│   ├── data/           # Data files (TOML, JSON, or Markdown)
│   ├── pages/          # Your site pages
│   ├── public/         # Static assets
│   └── templates/      # Templates for dynamic content
```

## Creating Your First Page

Create a file at `src/pages/index.html`:

```html
<!DOCTYPE html>
<html>
<head>
  <title>My Site</title>
</head>
<body>
  <h1>Hello World!</h1>
  <-Template{Posts} />
</body>
</html>
```

## Building Your Site

Run the build command:

```bash
simple build my-project
```

Your compiled site will be in the `dist/` directory!

## Development Mode

For live reloading during development:

```bash
simple dev my-project
```

This starts a local server and watches for changes.
