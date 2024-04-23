
import * as path from 'path';
import { workspace, ExtensionContext } from 'vscode';

import * as vscode from 'vscode'
import * as which from "which"
import * as os from "os"
import fetch from "node-fetch"
import * as fs from "fs"
import * as stream from "stream"
import {promisify} from "util"
import * as child_process from "child_process"

import {
	LanguageClient,
	LanguageClientOptions,
	Location,
	Position,
	ServerOptions,
	TransportKind,
	URI
} from 'vscode-languageclient/node';
import { unzip } from 'zlib';
import { downloadFile } from './util';

let client: LanguageClient;

const extId = "moxide"
const versionTag = "v0.0.20"

const releaseBaseUrl = "https://github.com/Feel-ix-343/markdown-oxide/releases/download"
export async function activate(context: ExtensionContext) {
	// The server is implemented in node
	// const serverModule = context.asAbsolutePath(
	// 	path.join('server', 'out', 'server.js')
	// );
	

	let findReferencesCmd = vscode.commands.registerCommand(`${extId}.findReferences`, findReferencesCmdImpl);


  let path = await languageServerPath(context);
  console.log(path)

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
			fileEvents: workspace.createFileSystemWatcher('**/*.md')
		}
	};

	// Create the language client and start the client.
	client = new LanguageClient(
		'markdown-oxide',
		'Markdown Oxide',
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


function serverBinName() {
  let platform = os.platform();
  if (platform == "win32") {
    return `markdown-oxide.exe`
  } else {
    return `markdown-oxide`
  }
}

async function languageServerPath(context: vscode.ExtensionContext) {
  // Check if in path
  let binName = serverBinName();
  let inPath = new Promise<string>((resolve, reject) => {
    which(binName, (err, path) => {
      if (err) {
        reject(err);
      }
      if (path === undefined) {
        reject(new Error('which return undefined path'));
      } else {
        resolve(path);
      }
    });
  });
  let resolved: string = await inPath.catch((_) => null);

  if (resolved) {
    return resolved
  }

  // Otherwise, check downloads and download if necessary
	const targetDir = vscode.Uri.joinPath(context.globalStorageUri, versionTag);
	const targetFile = vscode.Uri.joinPath(targetDir, serverBinName());


  try {
    await vscode.workspace.fs.stat(targetFile);
		console.log("Markdown Oxide is already downloaded");
  } catch {
    await downloadServerFromGH(context, targetDir, targetFile)

    console.log(targetFile.fsPath)

  }

	try {
		await vscode.workspace.fs.stat(targetFile);
		return targetFile.fsPath;
	} catch {
		console.error("Failed to download Markdown Oxide server binary");
		return null;
	}

}

/// Taken from https://github.com/artempyanykh/marksman-vscode/blob/main/src/extension.ts#L277
/// Also unzips the downloaded file
async function downloadServerFromGH(context: vscode.ExtensionContext, targetDir: vscode.Uri, targetFile: vscode.Uri) {
	await vscode.workspace.fs.createDirectory(targetDir);

  await vscode.window.withProgress({
    cancellable: false,
    title: `Downloading Markdown Oxide ${versionTag}`,
    location: vscode.ProgressLocation.Notification,
  }, async (progress, _) => {
			let lastPercent = 0;
			await downloadRelease(targetDir, targetFile, (percent) => {
				progress.report({ message: `${percent}%`, increment: percent - lastPercent });
				lastPercent = percent;
			});



  })
}

/// Taken from https://github.com/artempyanykh/marksman-vscode/blob/main/src/extension.ts
async function downloadRelease(targetDir: vscode.Uri, targetFile: vscode.Uri, onProgress: (progress: number) => void): Promise<void> {
	const tempName = (Math.round(Math.random() * 100) + 1).toString();
	const tempFile = vscode.Uri.joinPath(targetDir, tempName);
	const downloadUrl = releaseDownloadUrl();

	console.log(`Downloading from ${downloadUrl}; destination file ${tempFile.fsPath}`);
	const resp = await fetch(downloadUrl);

	if (!resp.ok) {
		console.error("Couldn't download the server binary");
		console.error({ body: await resp.text() });
		return;
	}

	const contentLength = resp.headers.get('content-length');
	if (contentLength === null || Number.isNaN(contentLength)) {
		console.error(`Unexpected content-length: ${contentLength}`);
		return;
	}
	let totalBytes = Number.parseInt(contentLength);
	console.log(`The size of the binary is ${totalBytes} bytes`);

	let currentBytes = 0;
	let reportedPercent = 0;
	resp.body.on('data', (chunk) => {
		currentBytes = currentBytes + chunk.length;
		let currentPercent = Math.floor(currentBytes / totalBytes * 100);
		if (currentPercent > reportedPercent) {
			onProgress(currentPercent);
			reportedPercent = currentPercent;
		}
	});

	const destStream = fs.createWriteStream(tempFile.fsPath);
	const downloadProcess = promisify(stream.pipeline);
	await downloadProcess(resp.body, destStream);

	console.log(`Downloaded the binary to ${tempFile.fsPath}`);
	await vscode.workspace.fs.rename(tempFile, targetFile);
	await fs.promises.chmod(targetFile.fsPath, 0o755);
}






function releaseDownloadUrl(): string {
  return releaseBaseUrl + "/" + versionTag + "/" + releaseBinName();
}

function releaseUrlExtension(): string {
  const platform = os.platform();
  if (platform == "win32") {
    return ".zip"
  } else {
    return ".tar.gz"
  }
}


function releaseBinName(): string {
	const platform = os.platform();
	const arch = os.arch();

  let arch_string: string = null;
  if (arch == "x64") {
    arch_string = "x86_64"
  } else if (arch == "arm64" || arch == "aarch64") {
    arch_string = "aarch64"
  }

  let platform_string: string = null;
  if (platform == "win32") {
    platform_string = "pc-windows-gnu"
  } else if (platform == "darwin") {
    platform_string = "apple-darwin"
  } else if (platform == "linux") {
    platform_string = "unknown-linux-gnu"
  }

  if (arch_string != null && platform_string != null) {

    return `markdown-oxide-${versionTag}-${arch_string}-${platform_string}`

  } else {
    throw new Error(`Unsupported platform: ${platform}; arch ${arch}`)
  }
}

