#!/usr/bin/env python3

from abc import ABCMeta, abstractmethod
from genericpath import exists
from os import chdir
import os
import subprocess
import sys
from typing import Literal, NoReturn

HOME = os.environ["HOME"]
GitRepository = Literal["github.com"]
Project = Literal["cargo"]


def error(*args) -> NoReturn:
    print("\033[91m" + " ".join(args) + "\033[0m", file=sys.stderr)
    exit(1)


def info(*args: str) -> None:
    print("\033[94m" + " ".join(args) + "\033[0m")


def exec(command: str) -> str:
    info(f"Executing '{command}'")
    res = subprocess.run(command.split(" "), capture_output=True)
    if res.returncode != 0:
        error(
            f"Command failed with code {res.returncode}\n", res.stderr.decode("utf-8")
        )
    return res.stdout.decode("utf-8").rstrip()


# class Module(metaclass=ABCMeta):
#     @abstractmethod
#     def should_load(self) -> bool:
#         "Should this module load"

#     @abstractmethod
#     def validate(self) -> None:
#         """Validate that the module has everything it needs to release"""

#     @abstractmethod
#     def release(self) -> None:
#         """Make release on given module"""


# class Github(Module):
#     def should_load(self):
#         url = exec("git remote get-url origin")
#         return "github.com" in url


def get_git_repository() -> GitRepository:
    url = exec("git remote get-url origin")
    if "github.com" in url:
        return "github.com"
    error("Could not find repository from", url)


def get_project() -> Project:
    if exists("Cargo.toml"):
        return "cargo"
    error("Could not find what kind of project this is")


def prepare_branch() -> None:
    master_branch = "master"
    if "master" not in exec("git branch"):
        master_branch = "main"
    remote = exec(f"git config branch.{master_branch}.remote")

    exec(f"git switch {master_branch}")
    exec(f"git pull {remote} {master_branch}")


def validate_github():
    exec("gh auth status")


def validate_cargo():
    if not exists(f"{HOME}/.cargo/credentials"):
        error(f"{HOME}/.cargo/credentials does not exist")

    exec("cargo test")
    exec("cargo build --release --locked")
    exec("cargo publish --dry-run")


def release():
    root = exec("git rev-parse --show-toplevel")
    info(f"Moving to root '{root}'")
    chdir(root)

    repo = get_git_repository()
    project = get_project()

    # if exec("git status --short") != "":
    #     error("Git branch is dirty")

    prepare_branch()

    if repo == "github.com":
        validate_github()

    if project == "cargo":
        validate_cargo()


release()
