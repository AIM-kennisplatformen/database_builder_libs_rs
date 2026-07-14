# SCEPA services

The services in [`docker-compose.yml`](docker-compose.yml) are grouped into two
profiles:

| Profile | Services |
| --- | --- |
| `typedb` | TypeDB and the TypeDB MCP server |
| `ingestion` | GROBID and TypeDB |

## Start the services

To start a profile use:

```sh
COMPOSE_PROFILES=<profile> docker compose up -d
```

To start multiple profile write multiple profiles in `<profile>` seperated with a `,`.

## Export TypeDB data

TypeDB exports a database as two files: a TypeQL schema file and a binary data
file. Install a TypeDB Console version compatible with the server, then make
sure the stack is running. The default local credentials configured in the
Compose file are `admin` / `password`.

List the available databases first:

```sh
typedb console \
  --address=localhost:1729 \
  --username=admin \
  --password=password \
  --tls-disabled \
  --command="database list"
```

Export a database by replacing `my_database` with its name:

```sh
typedb console \
  --address=localhost:1729 \
  --username=admin \
  --password=password \
  --tls-disabled \
  --command="database export <database> <dir>/schema.typeql <dir>/data.typedb"
```

The resulting `schema.typeql` and `data.typedb` files are written to the host,
outside the Docker volume, and can be copied or archived as a backup.

## Import TypeDB data

The target database must not already exist. You can use a new name when
restoring or migrating a database:

```sh
typedb console \
  --address=localhost:1729 \
  --username=admin \
  --password=password \
  --tls-disabled \
  --command="database import <database> <dir>/schema.typeql <dir>/data.typedb"
```

If the target name already exists, either choose another name or delete the
existing database first after confirming that it is no longer needed:

```sh
typedb console \
  --address=localhost:1729 \
  --username=admin \
  --password=password \
  --tls-disabled \
  --command="database delete <database>"
```

For more detail, see the [TypeDB database export and import
documentation](https://typedb.com/docs/maintenance-operation/database-export-import/)
and the [TypeDB Console installation
instructions](https://typedb.com/docs/home/install/console-cli/).
