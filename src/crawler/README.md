## Example configuration

```unit file (systemd)
[Unit]
Description = Blitz Dashboard Crawler
BindsTo = network-online.target postgresql.service redis.service
After = network-online.target postgresql.service redis.service

[Service]
ExecStart = /home/pi/bin/blitz-dashboard \
    --sentry-dsn=<Sentry-DSN> \
    -v \
    crawl \
    -a=<application-ID> \
    -d=postgres://user@host/database \
    --auto-min-offset \
    --n-buffered-batches=20 \
    --n-buffered-accounts=10 \
    --log-interval=60s \
    --stream-duration=5d \
    --redis-uri=redis+unix:///var/run/redis/redis-server.sock
WorkingDirectory = /home/pi
StandardOutput = journal
StandardError = journal
Restart = always
RestartSec = 5
User = pi

[Install]
WantedBy = multi-user.target
```

## Tuning

Wargaming.net API is limited at 20 requests per second for a server-side application. For the optimal performance try and utilise 19-20 RPS for the crawler service by tuning the few options:

### `--n-buffered-batches`

Defines how many batches of up to 100 accounts get crawled concurrently. For each batch there's at least 1 request per batch and 2 additional requests per each account which last battle time has changed. Increase this option one by one till you hit the maximal RPS without getting `REQUEST_LIMIT_EXCEEDED` errors yet. Values greater than `20` will most likely lead to the numerous errors. The value of `20` is actually a good try.

### `--n-buffered-accounts`

Defines how many accounts get updated in the database concurrently. Increase one by one till you get the stable maximal RPS. You're limited by a maximal number of active database connections.

## «Cold» start

In order to run the crawler, you'd need to fill in the database with some accounts. The web application automatically stores all viewed accounts in the database. But you can also scan all the account ID space and import existing accounts with `blitz-dashboard crawl-accounts`.

For example:

```shell
blitz-dashboard -v crawl-accounts --initialize-schema -d postgres://pi@localhost/yastatist -a <application-ID> --start-id 1 --end-id 150000000
```

This is a **very** slow process. On average, you'll be importing around 1M accounts per day.
