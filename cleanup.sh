#!/bin/zsh -e
redis-cli KEYS 'f::ru::*' | xargs redis-cli DEL
redis-cli DEL cf::vehicles
