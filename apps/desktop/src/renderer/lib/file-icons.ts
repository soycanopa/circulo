/**
 * File icon utilities — wraps file-icons-js for VSCode-style file type icons.
 *
 * Uses the Atom file-icons CSS font, providing icon + color class names
 * for any file path based on extension matching.
 */

import "file-icons-js/css/style.css"
import { getClassWithColor } from "file-icons-js"

export function getFileIconClass(path: string): string {
	return getClassWithColor(path) ?? ""
}
