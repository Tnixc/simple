# Simple Build tool

A simple build tool that assembles a static site from components, templates, and data. I used it to build v6 of https://tnixc.space

- [ ] Fix problem where there are 2 sources of truth for md frontmatter in the current way I do blog posts, will hopefully be fixed with proper md rendering.
- [ ] Handle port collisions.
- [ ] Unindent markdown element
- [ ] Make it so that it fails more strictly on <Template>.
- [ ] Handle failure to parse json.
- [ ] Even better error messages that point out the exact line and file.
- [ ] Keep some elements loaded to avoid reading from disk every time.
- [ ] Switch to using a .config file
- [ ] Proper escaping from {}, "", etc. (?)
- [ ] Implement MD rendering (external lib) from .md files. Not needed tho maybe?
- [ ] Look into testing (tons of edge cases)
- [ ] OG Image and meta generation, especially for markdown posts
- [ ] Give warnings for unused and not found components
- [ ] CSS scoping in components but waiting for @scope general support , so just use tailwind for now
- [ ] Type safety-ish warnings when template doesn't match data
- [ ] Cache what has changed with hashing so no need to rebuild if stuff is same. -> can massively speed it up if lots of templating.
- [ ] Improve flexibility w/ markdown syntax highlighting theme
- [x] Check for circular deps.
- [x] Watcher or HMR (it was not as complicated as I thought, hell yeah)
- [x] Actually good logs with color, status, etc.
- [x] <markdown> component
- [x] Commands such as dev, build, new.
- [x] Props and slots
- [x] Make it so it ignores commented out lines.
- [x] The error handling is abysmal. Mainly due to me using unwrap(), Result type and ? everywhere. - more work to be done but it's in an ok state for now
- [x] Speed tests.
- [x] Get templates working
- [x] Get components working
- [x] Copy files from /public to /dist


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

To use the above, you would run the following command, where target contains a folder `src`.

```sh
simple build /path/to/target
```

## Components

To use components in markup, do the following:

```html
<Component />
<!-- for a self closing component -->
<Layout><h1>Hello world</h1></Layout>
<!-- for a component which contains a <slot></slot> -->
```

If no content inside a slot component is provided, it will use the fallbacks inside the `<slot>` tags. To access components in folders, use `:`s to separate them like so:

```html
<Folder:Component />
```

## Templating

```html
<!-- e.g. in index.html -->
<-Template{Name} />
```

Think of this like a `for each`, where `Name` will be used to search for `src/data/Name.data.json` to populate instances of `src/templates/Name.template.html`

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

The `data.json` file must contain an array of objects with the keys for the template.

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

The contents inside the markdown tags need to be indented for now. Fix coming soon?

## Naming

Components, templates, and data must following `CamalCase`, not contain spaces, and cannot start with a number. They can contain underscores.


