import type { Compiler, Configuration } from "webpack"
import type { Configuration as DevServerConfiguration } from "webpack-dev-server"
import path from "node:path"
import HtmlWebpackPlugin from "html-webpack-plugin"
import MiniCssExtractPlugin from "mini-css-extract-plugin"
import WasmPackPlugin from "@wasm-tool/wasm-pack-plugin"
import CopyPlugin from "copy-webpack-plugin"
// @ts-ignore
import typogr from "typogr"

class TypogrHtmlPlugin {
    options: Object

    constructor(options = {}) {
        this.options = options;
    }

    apply(compiler: Compiler) {
        compiler.hooks.compilation.tap("TypogrHtmlPlugin", (compilation) => {
            HtmlWebpackPlugin.getHooks(compilation).beforeEmit.tapAsync(
                'TypogrHtmlPlugin',
                (data, callback) => {
                    try {
                        data.html = typogr.typogrify(data.html)

                        callback(null, data)
                    } catch (error) {
                        callback(error as any)
                    }
                }
            )
        })
    }
}

export default {
    entry: "./src/index.ts",
    output: {
        filename: "bundle.js",
        path: path.resolve(import.meta.dirname, "dist"),
    },
    module: {
        rules: [
            {
                test: /\.ya?ml$/,
                loader: "yaml-loader"
            },
            {
                test: /\.ts$/i,
                use: "ts-loader",
                exclude: path.resolve(import.meta.dirname, "node_modules"),
            },
            {
                test: /\.css$/i,
                use: [
                    MiniCssExtractPlugin.loader,
                    "css-loader",
                    "postcss-loader"
                ]
            }
        ]
    },
    resolve: {
        extensions: [".ts", ".js"],
    },
    plugins: [
        new HtmlWebpackPlugin({
            template: "./src/index.html",
            filename: "index.html"
        }),
        new TypogrHtmlPlugin(),
        new MiniCssExtractPlugin(),
        new WasmPackPlugin({
            crateDirectory: path.resolve(import.meta.dirname, "../core/terminals/web"),
            extraArgs: "--reference-types",
            // forceMode: "production"
        }),
        new CopyPlugin({
            patterns: [
                {
                    from: "public",
                    to: ""
                }
            ]
        })
    ],
    experiments: {
        asyncWebAssembly: true
    }
} satisfies Configuration & { devServer?: DevServerConfiguration }