import { createV3FixtureRepository, type JsonLoader, type V3ViewRepository } from "@/v3/contracts/fixtureRepository";

const jsonLoader: JsonLoader = async <T>(path: string): Promise<T> => {
  const resp = await fetch(path);
  if (!resp.ok) {
    throw new Error(`Failed to load fixture: ${path} (${resp.status})`);
  }
  return resp.json() as Promise<T>;
};

export const v3Repository: V3ViewRepository = createV3FixtureRepository(jsonLoader);
