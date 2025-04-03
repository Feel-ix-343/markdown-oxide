
import * as path from 'path';
import { workspace, ExtensionContext } from 'vscode';

import * as vscode from 'vscode'
import * as which from "which"
import * as os from "os"
import fetch from "node-fetch"
import * as fs from "fs"
import * as stream from "stream"
import {promisify} from "util"

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
const versionTag = "v0.22.0"

const releaseBaseUrl = "https://github.com/Feel-ix-343/markdown-oxide/releases/download"

// Define DateQuickPickItem interface
interface DateQuickPickItem extends vscode.QuickPickItem {
	dateString: string; // Date string to pass to the backend
}

// Helper function: Format date as YYYY-MM-DD
function formatDate(date: Date): string {
	// Format date as YYYY-MM-DD
	return date.toISOString().slice(0, 10);
}

// Get date object with specified offset days
function getDateWithOffset(days: number): Date {
	const date = new Date();
	date.setDate(date.getDate() + days);
	return date;
}

// Get specific weekday date (current week or relative week)
function getWeekday(dayOfWeek: number, weekOffset: number = 0): Date {
	const now = new Date();
	const currentDay = now.getDay(); // 0 = Sunday, 1 = Monday, ..., 6 = Saturday

	// Calculate days until target weekday
	const daysUntilTarget = (dayOfWeek + 7 - currentDay) % 7;

	// Calculate date
	const targetDate = new Date(now);
	targetDate.setDate(now.getDate() + daysUntilTarget + (weekOffset * 7));

	return targetDate;
}

// Generate common date options
function generateDateOptions(): DateQuickPickItem[] {
	const items: DateQuickPickItem[] = [];
	const weekdays = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
	const weekdayNames = ['sunday', 'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday'];

	// Today, Yesterday, Tomorrow
	items.push({
		label: `Today (${formatDate(new Date())})`,
		description: 'today',
		dateString: 'today'
	});
	items.push({
		label: `Yesterday (${formatDate(getDateWithOffset(-1))})`,
		description: 'yesterday',
		dateString: 'yesterday'
	});
	items.push({
		label: `Tomorrow (${formatDate(getDateWithOffset(1))})`,
		description: 'tomorrow',
		dateString: 'tomorrow'
	});

	// Days of current week
	for (let i = 0; i < 7; i++) {
		const date = getWeekday(i);
		items.push({
			label: `This ${weekdays[i]} (${formatDate(date)})`,
			description: `this ${weekdayNames[i]}`,
			dateString: `this ${weekdayNames[i]}`
		});
	}

	// Days of last week
	for (let i = 0; i < 7; i++) {
		const date = getWeekday(i, -1);
		items.push({
			label: `Last ${weekdays[i]} (${formatDate(date)})`,
			description: `last ${weekdayNames[i]}`,
			dateString: `last ${weekdayNames[i]}`
		});
	}

	// Days of next week
	for (let i = 0; i < 7; i++) {
		const date = getWeekday(i, 1);
		items.push({
			label: `Next ${weekdays[i]} (${formatDate(date)})`,
			description: `next ${weekdayNames[i]}`,
			dateString: `next ${weekdayNames[i]}`
		});
	}

	return items;
}

export async function activate(context: ExtensionContext) {
	// The server is implemented in node
	// const serverModule = context.asAbsolutePath(
	// 	path.join('server', 'out', 'server.js')
	// );
	

	let findReferencesCmd = vscode.commands.registerCommand(`${extId}.findReferences`, findReferencesCmdImpl);
	
	// Register semantic jump command
	let semanticJumpCmd = vscode.commands.registerCommand('markdown-oxide.semanticJump', async () => {
		if (!client) {
			vscode.window.showWarningMessage('Markdown Oxide language client is not ready yet.');
			return;
		}
		
		const dateOptions = generateDateOptions(); // Generate date options
    
		const quickPick = vscode.window.createQuickPick<DateQuickPickItem>();
		quickPick.items = dateOptions;
		quickPick.placeholder = "Select or enter date to jump to (e.g.: today, yesterday, next monday)...";
		quickPick.matchOnDescription = true; // Enable matching on description
		
		quickPick.onDidAccept(async () => {
			const selectedItem = quickPick.selectedItems[0];
			const dateStringToJump = selectedItem ? selectedItem.dateString : quickPick.value; // Get selected item or input value
			
			quickPick.hide(); // Hide first
			
			if (dateStringToJump) {
				try {
					// Send workspace/executeCommand request to LSP Server
					await client.sendRequest('workspace/executeCommand', {
						command: 'jump',
						arguments: [dateStringToJump]
					});
				} catch (error) {
					vscode.window.showErrorMessage(`Jump command execution failed: ${error}`);
					console.error("Error executing jump command:", error);
				}
			}
		});
		
		quickPick.onDidHide(() => quickPick.dispose()); // Clean up resources
		quickPick.show(); // Show the list
	});

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
	
	// 将命令处理器添加到订阅中
	context.subscriptions.push(findReferencesCmd, semanticJumpCmd);
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
    platform_string = "pc-windows-gnu.exe"
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

