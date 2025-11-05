---
title: Advanced Features in Simple
description: Exploring components, slots, and custom templates
date: Jan 25 2025
author: John Doe
tags: advanced, components, templates
readtime: 8 min
featured: true
---

# Advanced Features in Simple

Now that you're familiar with the basics, let's explore some of Simple's more advanced features.

## Components with Props

Components can accept props to make them more flexible:

```html
<Button color="blue" size="large">Click Me</Button>
```

In your component file:

```html
<button class="btn btn-${color} btn-${size}">
  <slot>Default Text</slot>
</button>
```

## Nested Components

You can organize components in folders:

```html
<Layout:Header />
<Layout:Footer />
<Common:Card title="My Card" />
```

## Template Entries with Frontmatter

The real power comes from combining templates with frontmatter. Instead of maintaining separate JSON files, you can extract all metadata directly from your markdown files!

### Before (the old way):

```json
{
  "title": "My Post",
  "description": "...",
  "link": "./content/my-post.html",
  "--entry-path": "Posts/my-post.md",
  "--result-path": "content/my-post.html"
}
```

### After (the new way):

```markdown
---
title: My Post
description: ...
---

Content here...
```

Much cleaner! The `--entry-path`, `--result-path`, and `link` fields are automatically generated.

## Custom Markdown Rendering

You can use the `<markdown>` component anywhere:

```html
<markdown>
  # This is rendered as markdown

  **Bold text** and *italic text*
</markdown>
```

## Tips and Tricks

1. **Use frame files** for consistent layouts across entries
2. **Keep components small** and focused on one thing
3. **Use frontmatter** for all your metadata needs
4. **Leverage slots** for flexible component content

Happy building!
