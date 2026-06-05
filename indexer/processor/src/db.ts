import postgres from 'postgres';

export function getDb(connectionString: string) {
  return postgres(connectionString, {
    ssl: false,
    max: 1,
    idle_timeout: 10,
    connect_timeout: 15
  });
}

export type Db = ReturnType<typeof getDb>;
