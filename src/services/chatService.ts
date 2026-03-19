import { invoke } from "@tauri-apps/api/core";

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
}

interface ChatResponse {
  reply: string;
}

export async function chatWithContent(
  articleText: string,
  history: ChatMessage[],
  userInput: string
): Promise<string> {
  const response = await invoke<ChatResponse>("chat_with_content", {
    articleText,
    history,
    userInput,
  });
  return response.reply;
}

export async function getChatHistory(contentId: string): Promise<ChatMessage[]> {
  return invoke<ChatMessage[]>("get_chat_history", { contentId });
}

export async function saveChatMessage(
  contentId: string,
  role: "user" | "assistant",
  message: string
): Promise<void> {
  await invoke("save_chat_message", { contentId, role, message });
}

export async function clearChatHistory(contentId: string): Promise<void> {
  await invoke("clear_chat_history", { contentId });
}
