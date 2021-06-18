#!/bin/sh
set -e

RELEASE=target/release
CROSS_RELEASE=xtarget/xtensa-esp8266-none-elf/release-embedded
DIST="$RELEASE"/dist
LIB_NATIVE="$DIST"/lib/native
LIB_ESP8266="$DIST"/lib/esp8266

mkdir "$DIST"
mkdir -p "$LIB_NATIVE" "$LIB_ESP8266"

cp "$RELEASE"/{compiler,editor} "$DIST"
cp $RELEASE/libruntime.a "$LIB_NATIVE"

cp "$CROSS_RELEASE"/libruntime.a "$LIB_ESP8266"
find "$CROSS_RELEASE" -name '*.x' -exec cp {} $DIST/lib/esp8266 \;
