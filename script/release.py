#!/usr/bin/env python3

from abc import ABCMeta, abstractmethod
from genericpath import exists
from os import chdir
import os
import re
import subprocess
import sys
from typing import Dict, List, NoReturn, Optional, Tuple
from urllib.request import Request, urlopen


def error(*args) -> NoReturn:
    print("\033[91m" + " ".join(args) + "\033[0m", file=sys.stderr)
    exit(1)


def warn(*args) -> None:
    print("\033[92m" + " ".join(args) + "\033[0m", file=sys.stderr)


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


def request(url: str, headers: Optional[Dict[str, str]] = None) -> Tuple[int, str]:
    if headers == None:
        headers = {}
    req = Request(url, headers=headers)
    with urlopen(req) as response:
        return response.status, str(response.read())


HOME = os.environ["HOME"]
VERSION = sys.argv[1]
pattern = "^[0-9]*\\.[0-9]*\\.[0-9]*$"
if re.match(pattern, VERSION) == None:
    error("No version given")


class Module(metaclass=ABCMeta):
    def name(self) -> str:
        return type(self).__name__

    @abstractmethod
    def should_load(self) -> bool:
        "Should this module load"

    @abstractmethod
    def link(self) -> Optional[str]:
        """Give a link to the project"""

    @abstractmethod
    def validate(self) -> None:
        """Validate that the module has everything it needs to release"""

    def pre_release(self) -> None:
        pass

    @abstractmethod
    def release(self) -> None:
        """Make release on given module"""


class Github(Module):
    def should_load(self):
        url = exec("git remote get-url origin")
        return "github.com" in url

    def link(self):
        return exec("gh browse -n")

    def validate(self):
        exec("gh auth status")

    def release(self):
        exec(f"gh release create {VERSION}")


class Aur(Module):
    def should_load(self):
        url = exec("git remote get-url origin")
        exec("git --no-pager shortlog -sne")

    def validate(self):
        exec("gh auth status")

    def release(self):
        exec(f"gh release create {VERSION}")


class Cargo(Module):
    def should_load(self):
        return exists("Cargo.toml")

    def link(self):
        with open("Cargo.toml", "r") as file:
            name = re.search("^name\\s*=.*", file.read(), re.MULTILINE)
            if name is None:
                error("Could not find crate name")
            name = str(name).split("=")[1].replace(" ", "")

        link = f"https://crates.io/crates/{name}"
        [_, content] = request(link, {"Accept": "text/html"})

        if "Not Found" in content:
            warn(f"{link} does not exist")

        return

    def validate(self):
        if not exists(f"{HOME}/.cargo/credentials"):
            error(f"{HOME}/.cargo/credentials does not exist")

        exec("cargo test")
        exec("cargo build --release --locked")
        # exec("cargo publish --dry-run")

    def pre_release(self) -> None:
        info("Updating version in Cargo.toml")
        with open("Cargo.toml", "r") as file:
            cargo_toml: str = file.read()
        cargo_toml = re.sub("^version\\s*=.*", f'version = "{VERSION}"', cargo_toml)
        with open("Cargo.toml", "w") as file:
            file.write(cargo_toml)
        exec("git add Cargo.toml")

    def release(self):
        exec(f"cargo publish")


def prepare_branch():
    """Returns the remote"""
    branch = "master"
    if "master" not in exec("git branch"):
        branch = "main"
    remote = exec(f"git config branch.{branch}.remote")

    exec(f"git switch {branch}")
    exec(f"git pull {remote} {branch}")
    return remote


def release():
    root = exec("git rev-parse --show-toplevel")
    info(f"Moving to root '{root}'")
    chdir(root)

    # if exec("git status --short") != "":
    #     error("Git branch is dirty")

    prepare_branch()

    modules: List[Module] = []
    for module in [Github(), Cargo()]:
        if module.should_load():
            info(f"Found {module.name()}")
            modules.append(module)

    for module in modules:
        module.validate()

    for module in modules:
        info(f"Will release on {module.name()} ({module.link()})")

    answer = input("Proceed with release? [Y/n]")
    if answer.lower() != "y":
        error("")

    # for module in modules:
    #     module.pre_release()

    # exec(f"git tag {VERSION}")
    # exec(f"git push {remote} --tags")
    # for module in modules:
    #     module.release()


release()
