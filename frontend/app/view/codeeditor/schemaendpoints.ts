// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import { getApi } from "@/app/store/global";
import { getWebServerEndpoint } from "@/util/endpoints";
import { isRustBackend } from "@/util/tauri-rpc";
import { invoke } from "@tauri-apps/api/core";

type EndpointInfo = {
    uri: string;
    fileMatch: Array<string>;
    schema: object;
};

// Schema name -> file match patterns (used by both modes)
const schemaFileMatches: Map<string, Array<string>> = new Map();
schemaFileMatches.set("settings", [`${getApi().getConfigDir()}/settings.json`]);
schemaFileMatches.set("connections", [`${getApi().getConfigDir()}/connections.json`]);
schemaFileMatches.set("aipresets", [`${getApi().getConfigDir()}/presets/ai.json`]);
schemaFileMatches.set("widgets", [`${getApi().getConfigDir()}/widgets.json`]);

// Build endpoint URLs for go-sidecar mode (HTTP fetch)
const endpointToSchema: Map<string, string> = new Map();
for (const name of schemaFileMatches.keys()) {
    endpointToSchema.set(`${getWebServerEndpoint()}/schema/${name}.json`, name);
}

async function getSchemaEndpointInfo(endpoint: string): Promise<EndpointInfo> {
    let schema: Object;
    const schemaName = endpointToSchema.get(endpoint) ?? endpoint;
    const fileMatch = schemaFileMatches.get(schemaName) ?? [];

    try {
        if (isRustBackend()) {
            // In rust-backend mode, use Tauri IPC instead of HTTP fetch
            schema = await invoke("get_schema", { schemaName });
        } else {
            const data = await fetch(endpoint);
            schema = await data.json();
        }
    } catch (e) {
        console.log("cannot find schema:", e);
        schema = {};
    }

    return {
        uri: endpoint,
        fileMatch,
        schema,
    };
}

const SchemaEndpoints = Array.from(endpointToSchema.keys());

export { getSchemaEndpointInfo, SchemaEndpoints };
