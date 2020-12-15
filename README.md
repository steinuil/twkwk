# twkwk

A barebones server for [TiddlyWiki 5](https://tiddlywiki.com/) which serves
the wiki on any path and handles saving and backups.

Backups are not automatically cleaned, so you might wanna set up a cron job
for that.

## Building and running

```
cargo build --release
twkwk --wiki-file ./wiki --backup-dir ./backups --port 8000
```

Command line options should be self-explainatory.
