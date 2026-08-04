# Save and Load context state data
> **Status:** Session-file persistence is not implemented by the current ForgeSwift, ForgeAndroid, or Rust public APIs. In-memory multi-turn context is supported while an engine remains alive; use `reset()` to clear it.

The notes below describe the earlier LLM Farm behavior and are retained for historical context.

* LLM Farm is now able to save a session to a special file and load it later to resume dialog with LLM.
* This option is enabled by default.
* If you want to disable the save option, you can do so in the "Advanced" section of the chat settings.
* If you want to reset the current state of the chat, you can use the "Reload dialog" and "Clear chat" buttons.
