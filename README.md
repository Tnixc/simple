# Simple Build tool

A simple build tool that assembles a static site from components, templates, and data. I used it to build v6 of https://tnixc.space

- [ ] Fix bugs (extremely buggy right now).
- [ ] Fix problem where there are 2 sources of truth for md in the current way I do blog posts, will hopefully be fixed with proper md rendering.
- [ ] Keep some elements loaded to avoid reading from disk every time.
- [ ] Switch to using a .config file
- [ ] Proper escaping from {}, "", etc.
- [ ] Check for circular deps.
- [ ] Implement MD rendering (external lib) from .md files. Not needed tho maybe?
- [ ] Look into testing (tons of edge cases)
- [ ] OG Image and meta generation, especially for markdown posts
- [ ] Give warnings for unused and not found components
- [ ] CSS scoping in components but waiting for @scope general support , so just use tailwind for now
- [ ] Type safety-ish warnings when template doesn't match data
- [ ] Cache what has changed with hashing so no need to rebuild if stuff is same. -> can massively speed it up if lots of templating.
- [ ] Improve flexibility w/ markdown syntax highlighting theme
- [ ] Watcher or HMR (HMR is too complex so probably not) -> livejs is not ideal and is quite broken rn.
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

will make version that actually uses [Web components](https://developer.mozilla.org/en-US/docs/Web/API/Web_components). In js/ts probably

# Docs

The file tree should look something like this:

```
.
└── src
   ├── components
   │  ├── Component.html
   │  └── Folder
   │     └── Component.html
   ├── data
   │  └── Something.json
   ├── pages
   │  ├── folder
   │  │  └── page.html
   │  └── index.html
   ├── public
   │  ├── image.png
   │  ├── folder
   │  │  └── image2.png
   └── templates
      └── Something.html
```

### Components

Components, and folders in /components must be in `PascalCase`. The first letter must be capitalized.

To use a component:

```html
<Component />
```

It searches from /components, so if a component is in a subfolder do this:

```html
<Folder:Component />
```

so don't use `:` in your component or folder names.

### Templates

The templates are stored in `/templates`, and the data for the corresponding template is in `/data`.

A data file should have the same name as the template file. Keep in mind the template file is the one that's being repeated.

To use:

```html
<-{Template} />
```

And in `templates/Template.template.html` and `data/Template.data.json`,

```html
<p>${something}</p>
```

Will match

```json
[{ "something": "a" }, { "something": "b" }]
```

You can use components in templates and vice versa. It recursively resolves each one so if a lot of places use a component it could become inefficient because it reads from disk every time. It also doesn't halt on circular dependencies.

---

It's very little code. Basically a glorified build script but I think it's pretty neat.

<details>
<summary>Random notes</summary>
![in Templates](https://github.com/Tnixc/simple/assets/85466117/e90a0455-320b-4d37-8ad2-2efd265171e3)
</details>
