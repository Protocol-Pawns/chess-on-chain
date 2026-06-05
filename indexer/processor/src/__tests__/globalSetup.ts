import { execSync } from 'child_process';
import { readFileSync } from 'fs';
import postgres from 'postgres';

const CONTAINER_NAME = 'chess-processor-test-pg';
const PORT = 54329;
const DB_URL = `postgres://test:test@127.0.0.1:${PORT}/chess_test`;

function waitForDb(url: string, attempts = 30): Promise<void> {
  return new Promise((resolve, reject) => {
    const tryConnect = (n: number) => {
      const db = postgres(url, { connect_timeout: 2 });
      db`SELECT 1`
        .then(() => db.end().then(resolve))
        .catch(() => {
          db.end().catch(() => {});
          if (n <= 0) return reject(new Error('postgres not ready'));
          setTimeout(() => tryConnect(n - 1), 500);
        });
    };
    tryConnect(attempts);
  });
}

function docker(cmd: string) {
  try {
    execSync(cmd, { stdio: 'ignore' });
  } catch {
    // container may not exist
  }
}

export default async function setup() {
  try {
    docker(`docker stop ${CONTAINER_NAME}`);
    docker(`docker rm -f ${CONTAINER_NAME}`);

    execSync(
      `docker run -d --name ${CONTAINER_NAME} -p ${PORT}:5432 -e POSTGRES_USER=test -e POSTGRES_PASSWORD=test -e POSTGRES_DB=chess_test postgres:16-alpine`,
      { stdio: 'pipe' }
    );

    await waitForDb(DB_URL);

    const migration = readFileSync(
      new URL('../../../migration.sql', import.meta.url),
      'utf8'
    );
    const db = postgres(DB_URL, { max: 1 });
    for (const stmt of migration
      .replace(/BEGIN;|COMMIT;/g, '')
      .split(';')
      .map(s => s.trim())
      .filter(Boolean)) {
      await db.unsafe(stmt);
    }
    await db.end();
  } catch (err) {
    console.error('failed to start test postgres:', err);
    console.error('make sure docker is running');
    process.exit(1);
  }

  process.env.TEST_DATABASE_URL = DB_URL;
  return async function teardown() {
    docker(`docker stop ${CONTAINER_NAME}`);
    docker(`docker rm -f ${CONTAINER_NAME}`);
  };
}
