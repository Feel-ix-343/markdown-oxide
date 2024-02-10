/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */

import * as path from 'path';
import { workspace, ExtensionContext } from 'vscode';

import * as vscode from 'vscode'

import {
	LanguageClient,
	LanguageClientOptions,
	Location,
	Position,
	ServerOptions,
	TransportKind,
	URI
} from 'vscode-languageclient/node';

let client: LanguageClient;

const extId = "moxide"

export function activate(context: ExtensionContext) {
	// The server is implemented in node
	// const serverModule = context.asAbsolutePath(
	// 	path.join('server', 'out', 'server.js')
	// );
	

	let findReferencesCmd = vscode.commands.registerCommand(`${extId}.findReferences`, findReferencesCmdImpl);


  let path = context.asAbsolutePath("../target/release/obsidian-ls")

	// If the extension is launched in debug mode then the debug server options are used
	// Otherwise the run options are used
	const serverOptions: ServerOptions = {
		command: path
		// run: { run: "/home/felix/coding/LargerIdeas/ObsidianLS/obsidian-ls/target/release/obsidian-ls", transport: TransportKind.ipc },
		// debug: {
		// 	module: serverModule,
		// 	transport: TransportKind.ipc,
		// }
	};

	// Options to control the language client
	const clientOptions: LanguageClientOptions = {
		// Register the server for plain text documents
		documentSelector: [{ scheme: 'file', language: 'markdown' }],
		synchronize: {
			// Notify the server about file changes to '.clientrc files contained in the workspace
			fileEvents: workspace.createFileSystemWatcher('**/*.md')
		}
	};

	// Create the language client and start the client.
	client = new LanguageClient(
		'languageServerExample',
		'Language Server Example',
		serverOptions,
		clientOptions
	);

	// Start the client. This will also launch the server
	client.start();
}

export function deactivate(): Thenable<void> | undefined {
	if (!client) {
		return undefined;
	}
	return client.stop();
}


// From https://github.com/artempyanykh/marksman-vscode/blob/41fe090146b998f1f58f340027c652c1d2f8525e/src/extension.ts#L45
type FindReferencesData = {
	uri: URI,
	position: Position,
	locations: Location[]
};


async function findReferencesCmdImpl(data: FindReferencesData) {
	if (client) {
		await vscode.commands.executeCommand(
			"editor.action.peekLocations",
			vscode.Uri.parse(data.uri),
			client.protocol2CodeConverter.asPosition(data.position),
			data.locations.map(client.protocol2CodeConverter.asLocation)
		)
	}
}


