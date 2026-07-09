# Plugins Manifest

Plugins are the cornerstone of this project, there a few types of categories but all programmed/scripted using KDL a function must be based on a plugin additions as well.

## UI Plugins

The backend is designed only to create create ui not dictate it, the backend is just a framework that works like html css for ui with respect to responsiveness and dynamic content. The UI plugins are the only way to create a ui for the backend, the backend will not dictate how the ui should look like or how it should be structured, it will only provide the tools to create a ui.

the backend for options or plugins themselves export tags used by UI plugins to design the ui.

what should be a rule is that a ui plugin should give you options categories and options that can be dynamic to mix and match different ui, lets say basic interface like hey the categories settings the frame in a sense can be one thing but with different categories a plugin should expose the options too mix and match different ui plugins to create a unique ui use.

## API Plugins

This plugins are used to grab metadata from web or other sources and dictate if they can be stored locally, this api can create custom categories of source but again be dictated by a media tag, VIDEO, AUDIO, IMAGE, BOOK,, image book (manga, comic, etc) and so on.

on the backend side different types of apis will be accommodated to grab metadata from different source structures, the backend will not dictate how the api should be structured or how it should be used, it will only provide the tools to use an api.

## Functionality Plugins

Functionality plugins interact with the backend to create an experience or define one by combining different UI and API plugins into a single category, or by exposing them inside tags for specific use cases. At the start of development, these are limited to the following:

- Torrent
- Web scraping
- Music streaming
- HTML embed (anything that can be embedded in a web page)
- Standard video streaming based on popular freemium web platforms, with respect to intellectual values

They also interact with the media tech and metadata API.
