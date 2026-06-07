import type { Handle } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
  const response = await resolve(event);

  response.headers.set(
    'Link',
    '<https://app.protocol-pawns.com/.well-known/ai/skill.md>; rel="agent-skill", <https://app.protocol-pawns.com/llms.txt>; rel="llms-txt"'
  );
  response.headers.set(
    'Content-Signal',
    'ai-train=yes, search=yes, ai-input=yes'
  );

  return response;
};
