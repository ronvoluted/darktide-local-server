# Darktide Local Server (server)

### Small local server for use with Warhammer 40,000: Darktide mods

This is the underlying server for the [Darktide Local Server mod](https://www.nexusmods.com/warhammer40kdarktide/mods/211). To use that library (which packages this server) in your mods refer to the [documentation in the repository's README](https://github.com/ronvoluted/darktide-mods/blob/main/DarktideLocalServer/README.md).

## Features

### Serving images

The only[*](#the-only-way-to-load-images) way to load images into the game is to fetch them via URL. Using the local server avoids having to host images online which would have to serve requests every day for every mod user, every time they launched the game, for every image loaded.

#### Usage

Send a GET request to `localhost:41012/image` with a `path` query parameter to the absolute path of a local image.

> [!note]
> The image's path **must** first be URL encoded.

For example, to return the local image `C:\ForTheEmperor!.jpg`:

[http://localhost:41012/image?path=C%3A%5CForTheEmperor%21.jpg](http://localhost:41012/image?path=C%3A%5CForTheEmperor%21.jpg)

### Running commands

In Lua we have access to `os.execute` and `io.popen` but both of them are blocking operations. There is a minimum 30ms threadlock even just for a a simple `echo For the Emperor!` each time you fire the call. Delegating command executions to the local server allows you to run these asynchronously.

#### Usage

Send a POST request to `localhost:41012/run` with a `command` string in the body. For example, to open a new text document named "For the Emperor":

```bash
curl http://localhost:41012/run -X POST -H "Content-Type: application/json" -d "{\"command\": \"notepad 'For the Emperor!'\"}"
```

If successful, the returned response contains a `success` boolean and the `pid` of the process created:

```json
{
	"pid": 35372,
	"success": true
}
```
If unsuccessful it will only return `success`:
```json
{
	"success": false
}
```

### Customising the port
To use a different port number than `41012`, create a `config.json` file next to `DarktideLocalServer.exe` with a `port` property. For example, to set the number to `1234`:

```json
{
	"port": 1234
}
```

## Notes

### Single instance

The server ensures only one instance of it is ever running so that multiple mods do not spawn multiple servers.

### Why is the default port 41012?

We want a port with low likelihood of clashing with other applications/devices. 40000 is a sensible start, but no doubt there are some 40K geeks around the world who've already set this port for something. Darktide takes place during the Indomitus Crusade, of which its latest Plague Wars incident is dated to the year 41,012.

### "The *only way to load images..."

Mainly because the texture compiling format hasn't been deciphered.

There **is** actually another way though... As a crazy experiment, I reconstructed images by rendering tens of thousands of 1x1 squares and colouring them from stored bitmap values. Besides being cumbersome, anything larger than ~300x300px would crash the game :)
