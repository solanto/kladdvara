import "@xterm/xterm/css/xterm.css"
import "./styles.css"

import { Terminal } from "@xterm/xterm"
import { FitAddon } from "@xterm/addon-fit"
import { Unicode11Addon } from "@xterm/addon-unicode11"
import { WasmVM as VM } from "../../core/terminals/web/pkg"
import terminalTheme from "./terminal-theme"

const programInput = document.getElementById("program-input")! as HTMLFormElement
const sources = Object.fromEntries(
    ["source--premade", "source--web", "source--upload"]
        .map(key =>
            [key, document.getElementById(key)!]
        )
)

function updateForm() {
    const selectedSource = new FormData(programInput).get("source")!.toString()

    for (const key in sources) sources[key].hidden = key != selectedSource
}

document.getElementById("source")?.addEventListener("change", updateForm)

const vm = new VM()

const terminalContainer = document.getElementById("terminal")!
const terminalFont: string = "DM Mono"
const terminalColorMagenta = "#a30062"
const terminalColorBrightMagenta = "#f3d3dd"
const terminal = new Terminal({
    allowProposedApi: true,
    cursorBlink: true,
    fontFamily: `${terminalFont}, monospace, sans-serif`,
    theme: {
        background: terminalTheme.colbg,
        black: terminalTheme.col0,
        blue: terminalTheme.col4,
        brightBlack: terminalTheme.col8,
        brightBlue: terminalTheme.col12,
        brightCyan: terminalTheme.col14,
        brightGreen: terminalTheme.col10,
        brightMagenta: terminalColorBrightMagenta,
        brightRed: terminalTheme.col9,
        brightWhite: terminalTheme.col15,
        brightYellow: terminalTheme.col11,
        cursor: terminalColorMagenta,
        cursorAccent: terminalColorBrightMagenta,
        cyan: terminalTheme.col6,
        foreground: terminalTheme.colfg,
        green: terminalTheme.col2,
        magenta: terminalColorMagenta,
        overviewRulerBorder: terminalTheme.colfgalt,
        red: terminalTheme.col1,
        selectionBackground: "highlight",
        selectionForeground: "highlighttext",
        white: terminalTheme.col7,
        yellow: terminalTheme.col3
    }
})

const terminalFit = new FitAddon()
terminal.loadAddon(terminalFit)
terminal.loadAddon(new Unicode11Addon())
terminal.unicode.activeVersion = "11"

function openTerminal() {
    terminal.open(terminalContainer)
    terminalFit.fit()
    window.addEventListener("resize", () => terminalFit.fit())

    terminal.writeln("\x1b[1m🍰 kladdvara: \x1b[0mvälkommen!")
}

document.fonts.load(`1rem ${terminalFont}`).catch(() => {
    terminal.options.fontFamily = "monospace, sans-serif"
}).finally(openTerminal)

terminal.onKey(({ key }) => {
    if (/^[\x20-\x7F]*$/.test(key)) vm.pushKey(key)
})

function maybeUpdateTerminal() {
    const output = vm.takeOutput()
    if (output) terminal.write(output)

    window.requestAnimationFrame(maybeUpdateTerminal)
}

window.requestAnimationFrame(maybeUpdateTerminal)

const SLICE_TIME_MILLISECONDS = 5

function runSlice() {
    const start = performance.now()

    let status = "continue"

    while (
        status == "continue" &&
        performance.now() - start < SLICE_TIME_MILLISECONDS
    ) status = vm.step()

    if (status != "halt") window.setTimeout(runSlice, 0)
}

const sourceHandlers: { [k: string]: (d: FormDataEntryValue | null) => Promise<Uint8Array> } = {
    "source--premade": async name => {
        const networkError = new Error("couldn't load program from jmeiners.com")

        try {
            const bytes = await (
                await fetch(`https://www.jmeiners.com/lc3-vm/supplies/${name}.obj`)
            ).bytes()

            if (bytes.length == 0) throw networkError

            return bytes
        } catch (error) {
            if ((error as Error).name == "NetworkError") throw networkError
            else throw error
        }
    },
    "source--web": async url => {
        if (!url) throw new Error("no URL provided")

        return fetch(url.toString())
            .then(response => response.bytes())
    },
    "source--upload": async file => {
        if (!file || !(file instanceof File) || file.size == 0)
            throw new Error("no file provided")

        return new Uint8Array(await file.arrayBuffer())
    }

}

const submitButton = programInput.querySelector("input[type=submit]")! as HTMLInputElement

function changeSubmitText(event: TransitionEvent) {
    submitButton.value = "Reload the page to start over"
    submitButton.classList.remove("text-hidden")
    submitButton.removeEventListener("transitionend", changeSubmitText)
}

async function startHandler(event: SubmitEvent) {
    event.preventDefault()

    const data = new FormData(programInput)
    const selectedSource = new FormData(programInput)
        .get("source")!
        .toString()

    const sourceDetails = data.get(selectedSource)

    terminal.writeln("\x1b[1m⏳ kladdvara: \x1b[0mloading…")

    await sourceHandlers[selectedSource](sourceDetails)
        .then(bytes => {
            programInput.removeEventListener("submit", startHandler)

            for (const fieldset of programInput.getElementsByTagName("fieldset"))
                fieldset.disabled = true

            submitButton.disabled = true
            submitButton.addEventListener("transitionend", changeSubmitText)
            submitButton.classList.add("text-hidden")

            vm.loadImage(bytes)

            terminal.writeln("\x1b[1m📥️ kladdvara: \x1b[0mprogram loaded")
            terminal.write("\r\n")

            runSlice()
        }).catch(error => terminal.writeln(
            `\x1b[1m⛔️ kladdvara: \x1b[0m${(error as Error).message}`
        ))

    terminal.focus()

    terminalContainer.scrollIntoView({
        behavior: "smooth",
        block: "center"
    })
}

programInput.addEventListener("submit", startHandler)

updateForm()