# Simple Build tool

- [ ] Commands such as `dev`, `build`, `new`.
- [ ] The error handling is abysmal. Mainly due to me using unwrap(), Result type and ? everywhere.
- [ ] Make it so it ignores commented out lines.
- [ ] Make it so that
- [ ] Check for circular deps
- [ ] Implement MD rendering (external lib)
- [ ] Look into testing
- [ ] Give warnings for unused and not found components
- [ ] Watcher or HMR (HMR is too complex so probably not)
- [ ] CSS scoping in components but waiting for [@scope general support](https://developer.mozilla.org/en-US/docs/Web/CSS/@scope)
- [ ] Type safety warnings when template doesn't match data
- [ ] Speed tests.
- [ ] Cache what has changed with hashing so no need to rebuild if stuff is same. -> can massively speed it up if lots of templating.
- [x] Get templates working
- [x] Get components working
- [x] Copy files from /public to /dist

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
<Folder/Component />
```

### Templates

The templates are stored in /templates, and the data for the corresponding template is in /data.

A data file should have the same name as the template file. Keep in mind the template file is the one that's being repeated.

To use:

```html
<-{Template} />
```

And in templates/Template.html and data/Template.json,

```html
<p>{something}</p>
```
Will match
```json
[
  { "something" : "a"},
  { "something" : "b"},
]
```

---

It's very little code. Basically a glorified build script but I think it's pretty neat.

<details>
<summary>Random notes</summary>
![in Templates](https://github.com/Tnixc/simple/assets/85466117/e90a0455-320b-4d37-8ad2-2efd265171e3)
</details>
