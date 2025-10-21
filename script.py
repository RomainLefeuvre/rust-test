import logging
from pathlib import Path
import shlex
import sys
from typing import TYPE_CHECKING, Any, Dict, List, Optional, Set, Tuple

# Configure logging to show in console
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[logging.StreamHandler(sys.stdout)]
)

# WARNING: do not import unnecessary things here to keep cli startup time under
# control
import click

from swh.core.cli import CONTEXT_SETTINGS, AliasedGroup
from swh.core.cli import swh as swh_cli_group

DEFAULT_CONFIG: Dict[str, Tuple[str, Any]] = {
    "graph": ("dict", {"cls": "local", "grpc_server": {}})
}
from swh.core.cli import swh as swh_cli_group


@swh_cli_group.group(name="graph", context_settings=CONTEXT_SETTINGS, cls=AliasedGroup)
@click.option(
    "--config-file",
    "-C",
    default=None,
    type=click.Path(
        exists=True,
        dir_okay=False,
    ),
    help="YAML configuration file",
)
@click.option(
    "--profile",
    type=str,
    help="Which Rust profile to use executables from, usually 'release' "
    "(the default) or 'debug'.",
)
@click.pass_context
def graph_cli_group(ctx, config_file, profile):
    """Software Heritage graph tools."""
    from swh.core import config

    ctx.ensure_object(dict)
    conf = config.read(config_file, DEFAULT_CONFIG)
    if "graph" not in conf:
        raise ValueError(
            'no "graph" stanza found in configuration file %s' % config_file
        )
    ctx.obj["config"] = conf

    if profile is not None:
        conf["profile"] = profile


@graph_cli_group.command(name="patate")
@click.option(
    "--force",
    is_flag=True,
    help="Regenerate files even if they already exist. Implies --ef",
)
@click.option(
    "--ef", is_flag=True, help="Regenerate .ef files even if they already exist"
)
@click.argument(
    "graph",
    type=click.Path(
        writable=True,
    ),
)
@click.pass_context
def reindex(ctx, force: bool, ef: bool, graph: str):
    """Reindex a SWH GRAPH to the latest graph format.

    GRAPH should be composed of the graph folder followed by the graph prefix
    (by default "graph") eg. "graph_folder/graph".
    """
    import os.path

    from swh.graph.shell import Rust

    ef = ef or force
    conf = ctx.obj["config"]
    if "profile" not in conf:
        conf["profile"] = "release"

    if (
        ef
        or not os.path.exists(f"{graph}.ef")    ):
        logging.info("Recreating Elias-Fano indexes on adjacency lists")
        Rust("swh-graph-index", "ef", f"{graph}", conf=conf).run()
       # Rust("swh-graph-index", "ef", f"{graph}-transposed", conf=conf).run()

    if (
        ef
        or not os.path.exists(f"{graph}-labelled.ef")
    ):
        with open(f"{graph}.nodes.count.txt", "rt") as f:
            node_count = f.read().strip()

        # ditto
        logging.info("Recreating Elias-Fano indexes on arc labels")
        Rust(
            "swh-graph-index",
            "labels-ef",
            f"{graph}-labelled",
            node_count,
            conf=conf,
        ).run()
   

    node2type_fname = f"{graph}.node2type.bin"
    if force or not os.path.exists(node2type_fname):
        logging.info("Creating node2type.bin")
        if os.path.exists(node2type_fname):
            os.unlink(node2type_fname)
        Rust("swh-graph-node2type", graph, conf=conf).run()

#add main

def main():
    return graph_cli_group(auto_envvar_prefix="SWH_GRAPH")


if __name__ == "__main__":
    main()