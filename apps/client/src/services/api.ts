const GATEWAY_URL = import.meta.env.VITE_GATEWAY_URL || "http://localhost:4000";

export async function fetchApi<T>(
  endpoint: string,
  options: RequestInit = {},
): Promise<T> {
  const response = await fetch(`${GATEWAY_URL}${endpoint}`, {
    headers: {
      "Content-type": "application/json",
      ...options.headers,
    },
    ...options,
  });


  if (!response.ok) {
    const text = await response.text();
    let errorMessage = `Erreur HTTP : ${response.status}`;

    // 2. On essaie de parser le texte en JSON (si c'en est)
    try {
      const json = JSON.parse(text);
      if (json?.error) {
        errorMessage = json.error;
      }
    } catch {
      // 3. Si ce n'est pas du JSON valide, on garde le texte brut s'il existe
      if (text) {
        errorMessage = text;
      }
    }
    throw new Error(errorMessage);
  }

  return response.json();
}
