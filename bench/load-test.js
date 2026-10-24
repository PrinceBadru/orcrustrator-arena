import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  scenarios: {
    constant_rate_naive: {
      executor: 'constant-arrival-rate',
      rate: 30, // 30 iterations per second
      timeUnit: '1s',
      duration: '30s',
      preAllocatedVUs: 20,
      maxVUs: 50,
      exec: 'testNaive',
    },
    constant_rate_handrolled: {
      executor: 'constant-arrival-rate',
      rate: 30,
      timeUnit: '1s',
      duration: '30s',
      preAllocatedVUs: 20,
      maxVUs: 50,
      exec: 'testHandRolled',
    },
    constant_rate_orcrustrator: {
      executor: 'constant-arrival-rate',
      rate: 30,
      timeUnit: '1s',
      duration: '30s',
      preAllocatedVUs: 20,
      maxVUs: 50,
      exec: 'testOrcrustrator',
    },
  },
  thresholds: {
    http_req_failed: ['rate<0.05'],
  },
};

export function testNaive() {
  const res = http.get('http://gateway-naive:8001/patients/p1/summary');
  check(res, { 'status is 200': (r) => r.status === 200 });
}

export function testHandRolled() {
  const res = http.get('http://gateway-handrolled:8002/patients/p1/summary');
  check(res, { 'status is 200': (r) => r.status === 200 });
}

export function testOrcrustrator() {
  const res = http.get('http://gateway-orcrustrator:8003/patients/p1/summary');
  check(res, { 'status is 200': (r) => r.status === 200 });
}
