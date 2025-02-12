import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Route | keyword', function (hooks) {
  setupApplicationTest(hooks);

  test('shows an empty list if the keyword does not exist on the server', async function (assert) {
    await visit('/keywords/foo');
    assert.equal(currentURL(), '/keywords/foo');
    assert.dom('[data-test-crate-row]').doesNotExist();
  });

  test('server error causes the error page to be shown', async function (assert) {
    this.server.get('/api/v1/crates', {}, 500);

    await visit('/keywords/foo');
    assert.equal(currentURL(), '/keywords/foo');
    assert.dom('[data-test-404-page]').exists();
    assert.dom('[data-test-title]').hasText('foo: Failed to load crates');
    assert.dom('[data-test-go-back]').doesNotExist();
    assert.dom('[data-test-try-again]').exists();
  });
});
