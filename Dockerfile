FROM rust:1-bookworm AS build

COPY --from=node:24-bookworm-slim /usr/local/bin/node /usr/local/bin/node
COPY --from=node:24-bookworm-slim /usr/local/lib/node_modules /usr/local/lib/node_modules
RUN ln -s ../lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm \
    && ln -s ../lib/node_modules/npm/bin/npx-cli.js /usr/local/bin/npx

WORKDIR /app

COPY ui/package.json ui/package-lock.json ui/.npmrc ui/
RUN npm --prefix ui ci

COPY . .

RUN cargo build --release --locked

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home dyad

WORKDIR /app
COPY --from=build /app/target/release/dyad /app/dyad
COPY --from=build /app/ui/build /app/ui/build

USER dyad
EXPOSE 8080
CMD ["/app/dyad"]
