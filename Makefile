## DO NOT EDIT!
# This file was provisioned by Terraform
# File origin: https://github.com/Arrow-air/tf-github/tree/main/src/templates/rust-all/Makefile.tftpl

DOCKER_NAME := arrow-lib-router

# Combined targets

help: .help-base .help-rust .help-python .help-cspell .help-markdown .help-editorconfig .help-toml
all: test build release rust-publish
build: rust-build

release: rust-release
test: rust-test-all cspell-test toml-test python-test md-test-links editorconfig-test
tidy: rust-tidy toml-tidy python-tidy editorconfig-tidy

include .make/base.mk
include .make/cspell.mk
include .make/markdown.mk
include .make/editorconfig.mk
include .make/toml.mk
include .make/rust.mk
include .make/python.mk
