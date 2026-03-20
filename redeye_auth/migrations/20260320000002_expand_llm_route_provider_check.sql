ALTER TABLE llm_routes
DROP CONSTRAINT IF EXISTS llm_routes_provider_check;

ALTER TABLE llm_routes
ADD CONSTRAINT llm_routes_provider_check
CHECK (provider IN ('openai', 'anthropic', 'gemini'));
